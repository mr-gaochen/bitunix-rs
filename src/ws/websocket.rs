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

use super::types::{MessageCallback, MessageHandler};

// 配置常量
const HEARTBEAT_INTERVAL: u64 = 20; // 20秒心跳
const SUBSCRIBE_REFRESH_INTERVAL: u64 = 60; // 60秒刷新订阅（如果需要）
const INITIAL_RETRY_DELAY: u64 = 5;
const MAX_RETRY_DELAY: u64 = 60;

// 定义 WebSocket 流类型
type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

// 连接函数：只负责建立连接，不负责创建无关的 channel
async fn connect_websocket(wss_domain: &str) -> Result<WsStream> {
    let ws_url = format!("wss://{}/public/", wss_domain);
    println!("正在连接 BitUnix: {}", ws_url);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    println!("BitUnix 连接成功");
    Ok(ws_stream)
}

// 订阅逻辑
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

    println!("发送订阅: {}", subscribe_msg);

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

async fn run_internal(
    wss_domain: &str,
    interval_str: &str,
    symbol: &str,
    handler: Option<Arc<dyn MessageHandler>>,
    callback: Option<MessageCallback>,
) -> Result<()> {
    let mut retry_delay = INITIAL_RETRY_DELAY;

    // 外部循环：负责断线重连
    loop {
        match connect_websocket(wss_domain).await {
            Ok(ws_stream) => {
                // 连接成功，重置重试延迟
                retry_delay = INITIAL_RETRY_DELAY;

                // 拆分读写流
                let (write_half, mut read_half) = ws_stream.split();
                let write = Arc::new(Mutex::new(write_half));

                // 1. 立即发起订阅
                {
                    let mut writer = write.lock().await;
                    if let Err(e) = subscribe_channel(&mut *writer, interval_str, symbol).await {
                        println!("BitUnix 初始订阅失败: {:?}, 准备重连...", e);
                        continue; // 订阅失败直接重连
                    }
                }

                // 定义定时器 (关键修复：使用 interval 替代 sleep)
                let mut heartbeat_timer = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
                heartbeat_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

                // 如果确实需要定时重新订阅（有些交易所不需要），保留这个
                let mut sub_refresh_timer = interval(Duration::from_secs(SUBSCRIBE_REFRESH_INTERVAL));
                sub_refresh_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

                // 内部循环：处理消息和心跳
                loop {
                    select! {
                        // 1. 心跳定时器 (在 select 外部定义，不会被重置)
                        _ = heartbeat_timer.tick() => {
                            let ping_msg = json!({ "op": "ping", "ping": Utc::now().timestamp() }).to_string();
                            // println!("发送心跳: {}", ping_msg); // 根据需要开启日志
                            let mut writer = write.lock().await;
                            if let Err(e) = writer.send(Message::Text(ping_msg)).await {
                                println!("BitUnix 发送心跳失败: {:?}，断开连接", e);
                                break; // 跳出内部循环，触发外部重连
                            }
                        }

                        // 2. 定时刷新订阅 (如果 API 要求保持活跃)
                        _ = sub_refresh_timer.tick() => {
                             let mut writer = write.lock().await;
                             // 忽略错误，仅仅打印日志，不一定要断开
                             if let Err(e) = subscribe_channel(&mut *writer, interval_str, symbol).await {
                                 println!("BitUnix 定时订阅刷新警告: {:?}", e);
                             }
                        }

                        // 3. 接收消息
                        msg = read_half.next() => {
                            match msg {
                                Some(Ok(message)) => {
                                    match message {
                                        Message::Text(text) => {
                                            // 处理业务逻辑
                                            if let Some(ref h) = handler {
                                                h.handle(&text).await;
                                            }
                                            if let Some(ref cb) = callback {
                                                cb(&text).await;
                                            }
                                        },
                                        Message::Binary(bin) => {
                                            // 有些交易所偶尔会发 gzip 压缩的二进制数据
                                            println!("收到二进制消息 (未处理): {} bytes", bin.len());
                                        },
                                        Message::Ping(ping) => {
                                            // 响应标准 WebSocket Ping (Tungstenite 默认会自动处理 Pong，但显式处理更稳妥)
                                            let mut writer = write.lock().await;
                                            if let Err(e) = writer.send(Message::Pong(ping)).await {
                                                println!("发送 Pong 失败: {:?}", e);
                                            }
                                        },
                                        Message::Pong(_) => {
                                            // 收到服务端的 Pong，说明连接健康
                                        },
                                        Message::Close(_) => {
                                            println!("服务端关闭了连接");
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                Some(Err(e)) => {
                                    println!("读取 WebSocket 错误: {:?}，准备重连", e);
                                    break;
                                }
                                None => {
                                    println!("WebSocket 流结束 (EOF)，准备重连");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("连接建立失败: {:?}", e);
            }
        }

        // 重连策略：指数退避
        println!("将在 {} 秒后重试...", retry_delay);
        sleep(Duration::from_secs(retry_delay)).await;

        retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
    }
}