use anyhow::anyhow;
use anyhow::Result;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, Mutex},
    time::{interval, sleep, Duration, MissedTickBehavior},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, instrument, warn}; // 引入 tracing 宏

use super::types::{MessageCallback, MessageHandler};

const HEARTBEAT_INTERVAL: u64 = 20;
const INITIAL_RETRY_DELAY: u64 = 5;
const MAX_RETRY_DELAY: u64 = 60;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

// 建立连接
async fn connect_websocket(wss_domain: &str) -> Result<WsStream> {
    let ws_url = format!("wss://{}/public/", wss_domain);
    debug!(url = %ws_url, "正在建立 WebSocket 连接"); // debug 级别，平时不显示
    let (ws_stream, _) = connect_async(&ws_url).await?;
    info!("BitUnix WebSocket 连接成功");
    Ok(ws_stream)
}

// 发送订阅
async fn subscribe_channel<S>(write: &mut S, interval: &str, symbol: &str) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    let subscribe_msg = json!({
        "op": "subscribe",
        "args": [{
            "symbol": symbol,
            "ch": interval,
        }]
    })
    .to_string();

    info!(%symbol, %interval, "发送订阅指令"); // info 级别，关键操作
    debug!(payload = %subscribe_msg, "订阅报文详情");

    write
        .send(Message::Text(subscribe_msg))
        .await
        .map_err(|e| anyhow!("订阅发送失败: {:?}", e))
}

pub async fn run_with_handler(
    wss_domain: &str,
    interval: &str,
    symbol: &str,
    handler: Arc<dyn MessageHandler>,
) -> Result<()> {
    run_internal(wss_domain, interval, symbol, Some(handler), None).await
}

pub async fn run_with_callback(
    wss_domain: &str,
    interval: &str,
    symbol: &str,
    callback: MessageCallback,
) -> Result<()> {
    run_internal(wss_domain, interval, symbol, None, Some(callback)).await
}

// 使用 instrument 自动挂载上下文：日志会自动带上 symbol 和 interval 字段
#[instrument(skip(handler, callback), fields(symbol = %symbol, interval = %interval_str))]
async fn run_internal(
    wss_domain: &str,
    interval_str: &str,
    symbol: &str,
    handler: Option<Arc<dyn MessageHandler>>,
    callback: Option<MessageCallback>,
) -> Result<()> {
    let mut retry_delay = INITIAL_RETRY_DELAY;

    loop {
        match connect_websocket(wss_domain).await {
            Ok(ws_stream) => {
                // 连接成功，重置重试延迟
                retry_delay = INITIAL_RETRY_DELAY;

                let (write_half, mut read_half) = ws_stream.split();
                let write = Arc::new(Mutex::new(write_half));

                // 1. 连接后立即订阅
                {
                    let mut writer = write.lock().await;
                    if let Err(e) = subscribe_channel(&mut *writer, interval_str, symbol).await {
                        error!(error = ?e, "初始订阅失败，准备重连");
                        continue;
                    }
                }

                // 2. 心跳定时器 (使用 interval 保证准时)
                let mut heartbeat_timer = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
                heartbeat_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

                loop {
                    select! {
                        _ = heartbeat_timer.tick() => {
                            let ping_msg = json!({ "op": "ping", "ping": Utc::now().timestamp() }).to_string();
                            // debug!("发送心跳 Ping"); // 只有在 RUST_LOG=debug 时才显示心跳日志
                            let mut writer = write.lock().await;
                            if let Err(e) = writer.send(Message::Text(ping_msg)).await {
                                error!(error = ?e, "心跳发送失败，断开连接");
                                break;
                            }
                        }

                        msg = read_half.next() => {
                            match msg {
                                Some(Ok(message)) => {
                                    match message {
                                        Message::Text(text) => {
                                            // 收到消息，处理回调
                                            // trace!(len = text.len(), "收到文本消息"); // 只有极详细模式才打印
                                            if let Some(ref h) = handler {
                                                h.handle(&text).await;
                                            }
                                            if let Some(ref cb) = callback {
                                                cb(&text).await;
                                            }
                                        }
                                        Message::Pong(_) => {
                                            debug!("收到服务端 Pong");
                                        }
                                        Message::Close(frame) => {
                                            warn!(?frame, "服务端关闭了连接");
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                Some(Err(e)) => {
                                    error!(error = ?e, "WebSocket 读取错误");
                                    break;
                                }
                                None => {
                                    warn!("WebSocket 连接流结束 (EOF)");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!(error = ?e, "连接建立失败");
            }
        }

        warn!("将在 {} 秒后重试连接...", retry_delay);
        sleep(Duration::from_secs(retry_delay)).await;
        retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
    }
}
