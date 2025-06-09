use anyhow::anyhow;
use anyhow::Result;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, Mutex},
    time::{sleep, Duration},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::types::{MessageCallback, MessageHandler};

const HEARTBEAT_INTERVAL: u64 = 20;
const RETRY_DELAY: u64 = 5;
const MAX_RETRY_ATTEMPTS: u32 = 10;
const MAX_RETRY_DELAY: u64 = 60;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect_websocket(
    wss_domain: &str,
) -> Result<(WsStream, mpsc::Sender<Message>, mpsc::Receiver<Message>)> {
    let ws_url = format!("wss://{}/public/", wss_domain);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (tx, rx) = mpsc::channel(100);
    Ok((ws_stream, tx, rx))
}

async fn subscribe_channel<S>(write: &mut S, interval: &str, symbol: &str) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Debug, // 加上这句约束
{
    let subscribe_msg = json!({
        "op": "subscribe",
        "args": {
            "symbol":symbol,
            "ch": interval,
        }
    })
    .to_string();

    println!("订阅消息:{:?}", subscribe_msg);

    write
        .send(Message::Text(subscribe_msg))
        .await
        .map_err(|e| anyhow!("【gate】订阅消息发送失败: {:?}", e))
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
    interval: &str,
    symbol: &str,
    handler: Option<Arc<dyn MessageHandler>>,
    callback: Option<MessageCallback>,
) -> Result<()> {
    println!("初始化 BitUnix WebSocket...");
    let mut retry_count = 0;
    let mut retry_delay = RETRY_DELAY;

    loop {
        match connect_websocket(wss_domain).await {
            Ok((ws_stream, _tx, mut rx)) => {
                let (write_half, mut read_half) = ws_stream.split();
                let write = Arc::new(Mutex::new(write_half));

                {
                    let mut writer = write.lock().await;
                    if let Err(e) = subscribe_channel(&mut *writer, interval, symbol).await {
                        println!("BitUnix 订阅失败: {:?}", e);
                        continue;
                    }
                }

                retry_count = 0;
                retry_delay = RETRY_DELAY;

                let write_clone_heartbeat = Arc::clone(&write);
                let write_clone_subscribe = Arc::clone(&write);

                loop {
                    select! {
                        // 心跳机制
                        _ = sleep(Duration::from_secs(HEARTBEAT_INTERVAL)) => {
                            let ping_msg = json!({ "op": "ping", "ping": Utc::now().timestamp() }).to_string();
                            let mut writer = write_clone_heartbeat.lock().await;
                            if let Err(e) = writer.send(Message::Text(ping_msg)).await {
                                println!("BitUnix 发送心跳失败: {:?}", e);
                                break;
                            }
                        }

                        // 定时订阅刷新
                        _ = sleep(Duration::from_secs(60)) => {
                            let mut writer = write_clone_subscribe.lock().await;
                            if let Err(e) = subscribe_channel(&mut *writer, interval, symbol).await {
                                println!("BitUnix 定时订阅失败: {:?}", e);
                            }
                        }

                        // 后台消息发出
                        Some(msg) = rx.recv() => {
                            let mut writer = write.lock().await;
                            if let Err(e) = writer.send(msg).await {
                                println!("BitUnix 发送消息失败: {:?}", e);
                                break;
                            }
                        }

                        // 接收服务端消息
                        Some(msg) = read_half.next() => {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Some(ref h) = handler {
                                        h.handle(&text).await;
                                    }
                                    if let Some(ref cb) = callback {
                                        cb(&text).await;
                                    }
                                }
                                Ok(_) => {} // 非文本消息忽略
                                Err(e) => {
                                    println!("BitUnix 接收 WebSocket 消息失败: {:?}", e);
                                    break;
                                }
                            }
                        }

                        else => break, // 所有流均结束，退出
                    }
                }
            }

            Err(e) => {
                println!("BitUnix 连接失败: {:?}", e);
            }
        }

        retry_count += 1;
        if retry_count >= MAX_RETRY_ATTEMPTS {
            println!("BitUnix 已达到最大重试次数，退出。");
            break;
        }

        println!("BitUnix {} 秒后重试连接...", retry_delay);
        sleep(Duration::from_secs(retry_delay)).await;
        retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
    }

    Ok(())
}
