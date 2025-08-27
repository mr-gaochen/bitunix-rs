use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::json;
use sha2::Sha256;
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, Mutex},
    time::{sleep, Duration},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::{
    model::position_event::PositionEvent,
    types::{MessageCallback, MessageHandler},
};

const HEARTBEAT_INTERVAL: u64 = 20;
const RETRY_DELAY: u64 = 5;
const MAX_RETRY_ATTEMPTS: u32 = 10;
const MAX_RETRY_DELAY: u64 = 60;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// 仓位推送回调
pub type PositionCallback = Arc<dyn Fn(PositionEvent) + Send + Sync>;

/// 生成随机 nonce
fn generate_nonce() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// 生成签名
fn generate_signature(api_key: &str, secret_key: &str, timestamp: i64, nonce: &str) -> String {
    let msg = format!("{}{}{}", api_key, timestamp, nonce);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(msg.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// WebSocket 登录
async fn login<S>(write: &mut S, api_key: &str, secret_key: &str) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    let timestamp = Utc::now().timestamp_millis();
    let nonce = generate_nonce();
    let sign = generate_signature(api_key, secret_key, timestamp, &nonce);

    let login_msg = json!({
        "op": "login",
        "args": [{
            "apiKey": api_key,
            "timestamp": timestamp,
            "nonce": nonce,
            "sign": sign
        }]
    })
    .to_string();

    println!("发送登录请求: {}", login_msg);

    write
        .send(Message::Text(login_msg))
        .await
        .map_err(|e| anyhow!("【bitunix】登录消息发送失败: {:?}", e))
}

/// 建立 WebSocket 连接
async fn connect_websocket(
    wss_domain: &str,
) -> Result<(WsStream, mpsc::Sender<Message>, mpsc::Receiver<Message>)> {
    let ws_url = format!("wss://{}/public/", wss_domain);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (tx, rx) = mpsc::channel(100);
    Ok((ws_stream, tx, rx))
}

/// 订阅频道
async fn subscribe_channel<S>(write: &mut S, channel: &str, symbol: Option<&str>) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    let args = if let Some(sym) = symbol {
        json!([{
            "symbol": sym,
            "ch": channel,
        }])
    } else {
        json!([{
            "ch": channel,
        }])
    };

    let subscribe_msg = json!({
        "op": "subscribe",
        "args": args
    })
    .to_string();

    println!("订阅消息: {}", subscribe_msg);

    write
        .send(Message::Text(subscribe_msg))
        .await
        .map_err(|e| anyhow!("【bitunix】订阅消息发送失败: {:?}", e))
}

/// 对外公开：带处理器 + position 回调
pub async fn run_with_handler(
    wss_domain: &str,
    interval: &str,
    symbol: &str,
    api_key: &str,
    secret_key: &str,
    handler: Arc<dyn MessageHandler>,
    position_callback: Option<PositionCallback>,
) -> Result<()> {
    run_internal(
        wss_domain,
        interval,
        symbol,
        api_key,
        secret_key,
        Some(handler),
        None,
        position_callback,
    )
    .await
}

/// 对外公开：带回调 + position 回调
pub async fn run_with_callback(
    wss_domain: &str,
    interval: &str,
    symbol: &str,
    api_key: &str,
    secret_key: &str,
    callback: MessageCallback,
    position_callback: Option<PositionCallback>,
) -> Result<()> {
    run_internal(
        wss_domain,
        interval,
        symbol,
        api_key,
        secret_key,
        None,
        Some(callback),
        position_callback,
    )
    .await
}

/// 主逻辑
async fn run_internal(
    wss_domain: &str,
    interval: &str,
    symbol: &str,
    api_key: &str,
    secret_key: &str,
    handler: Option<Arc<dyn MessageHandler>>,
    callback: Option<MessageCallback>,
    position_callback: Option<PositionCallback>,
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

                    // 登录
                    if let Err(e) = login(&mut *writer, api_key, secret_key).await {
                        println!("BitUnix 登录失败: {:?}", e);
                        continue;
                    }

                    // 订阅行情
                    if let Err(e) = subscribe_channel(&mut *writer, interval, Some(symbol)).await {
                        println!("BitUnix 订阅行情失败: {:?}", e);
                        continue;
                    }

                    // 订阅仓位
                    if let Err(e) = subscribe_channel(&mut *writer, "position", None).await {
                        println!("BitUnix 订阅仓位失败: {:?}", e);
                        continue;
                    }
                }

                retry_count = 0;
                retry_delay = RETRY_DELAY;

                let write_clone_heartbeat = Arc::clone(&write);
                let write_clone_subscribe = Arc::clone(&write);

                loop {
                    select! {
                        // 心跳
                        _ = sleep(Duration::from_secs(HEARTBEAT_INTERVAL)) => {
                            let ping_msg = json!({ "op": "ping", "ping": Utc::now().timestamp() }).to_string();
                            let mut writer = write_clone_heartbeat.lock().await;
                            if let Err(e) = writer.send(Message::Text(ping_msg)).await {
                                println!("BitUnix 发送心跳失败: {:?}", e);
                                break;
                            }
                        }

                        // 定时刷新订阅
                        _ = sleep(Duration::from_secs(60)) => {
                            let mut writer = write_clone_subscribe.lock().await;
                            if let Err(e) = subscribe_channel(&mut *writer, interval, Some(symbol)).await {
                                println!("BitUnix 定时订阅行情失败: {:?}", e);
                            }
                            if let Err(e) = subscribe_channel(&mut *writer, "position", None).await {
                                println!("BitUnix 定时订阅仓位失败: {:?}", e);
                            }
                        }

                        // 后台发消息
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
                                    // 判断是否是仓位推送
                                    if text.contains("\"ch\":\"position\"") {
                                        if let Some(cb) = &position_callback {
                                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                                                if let Some(data_array) = parsed["data"].as_array() {
                                                    for d in data_array {
                                                        if let Ok(event) = serde_json::from_value::<PositionEvent>(d.clone()) {
                                                            cb(event);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // 行情处理
                                    if let Some(ref h) = handler {
                                        h.handle(&text).await;
                                    }
                                    if let Some(ref cb) = callback {
                                        cb(&text).await;
                                    }
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    println!("BitUnix 接收消息失败: {:?}", e);
                                    break;
                                }
                            }
                        }

                        else => break,
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
