use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::{
    select,
    sync::mpsc,
    time::{interval, sleep, Duration, MissedTickBehavior},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message,
    MaybeTlsStream,
    WebSocketStream
};

use super::types::{MessageCallback, MessageHandler};

const HEARTBEAT_INTERVAL: u64 = 20;
const RETRY_DELAY: u64 = 5;
const MAX_RETRY_ATTEMPTS: u32 = 15;
const MAX_RETRY_DELAY: u64 = 60;

// --- 基础工具函数 ---

fn generate_nonce() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

fn generate_signature(api_key: &str, secret_key: &str, timestamp: i64, nonce: &str) -> String {
    let first_input = format!("{}{}{}", nonce, timestamp, api_key);
    let mut hasher = Sha256::new();
    hasher.update(first_input.as_bytes());
    let first_hash = format!("{:x}", hasher.finalize());

    let second_input = format!("{}{}", first_hash, secret_key);
    let mut hasher2 = Sha256::new();
    hasher2.update(second_input.as_bytes());
    format!("{:x}", hasher2.finalize())
}

// --- 核心逻辑优化 ---

/// 登录并订阅所有指定的频道
async fn login_and_subscribe(
    tx: &mpsc::Sender<Message>,
    api_key: &str,
    secret_key: &str,
    channels: &[String],
) -> Result<()> {
    // 1. 登录
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
    }).to_string();

    tx.send(Message::Text(login_msg)).await
        .map_err(|e| anyhow!("发送登录请求失败: {:?}", e))?;

    // 2. 批量订阅频道
    for channel in channels {
        let sub_msg = json!({
            "ch": channel,
            "ts": Utc::now().timestamp_millis(),
        }).to_string();

        println!("正在订阅频道: {}", channel);
        tx.send(Message::Text(sub_msg)).await
            .map_err(|e| anyhow!("订阅频道 {} 失败: {:?}", channel, e))?;
    }

    Ok(())
}

/// 兼容旧接口，使用 handler
pub async fn run_with_handler(
    wss_domain: &str,
    api_key: &str,
    secret_key: &str,
    channels: Vec<String>, // 新增：允许传入多个频道
    handler: Arc<dyn MessageHandler>,
) -> Result<()> {
    run_internal(wss_domain, api_key, secret_key, channels, Some(handler), None).await
}

/// 兼容旧接口，使用 callback
pub async fn run_with_callback(
    wss_domain: &str,
    api_key: &str,
    secret_key: &str,
    channels: Vec<String>, // 新增：允许传入多个频道
    callback: MessageCallback,
) -> Result<()> {
    run_internal(wss_domain, api_key, secret_key, channels, None, Some(callback)).await
}

async fn run_internal(
    wss_domain: &str,
    api_key: &str,
    secret_key: &str,
    channels: Vec<String>,
    handler: Option<Arc<dyn MessageHandler>>,
    callback: Option<MessageCallback>,
) -> Result<()> {
    let ws_url = format!("wss://{}/private/", wss_domain);
    let mut retry_count = 0;
    let mut retry_delay = RETRY_DELAY;

    loop {
        println!("连接 BitUnix WebSocket: {}", ws_url);

        let connect_res = connect_async(&ws_url).await;
        let (ws_stream, _) = match connect_res {
            Ok(v) => v,
            Err(e) => {
                println!("连接失败: {:?}, {}秒后重试", e, retry_delay);
                sleep(Duration::from_secs(retry_delay)).await;
                retry_count += 1;
                retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                if retry_count >= MAX_RETRY_ATTEMPTS { break; }
                continue;
            }
        };

        retry_count = 0;
        retry_delay = RETRY_DELAY;

        let (mut ws_write, mut ws_read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<Message>(200); // 增加缓存容量

        // 登录并订阅传入的所有频道
        if let Err(e) = login_and_subscribe(&tx, api_key, secret_key, &channels).await {
            println!("初始化失败: {:?}", e);
            sleep(Duration::from_secs(RETRY_DELAY)).await;
            continue;
        }

        // 后台写入任务
        let mut write_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = ws_write.send(msg).await {
                    println!("WS 写入失败: {:?}", e);
                    break;
                }
            }
        });

        let mut hb_interval = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        hb_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        println!("BitUnix 运行中，已订阅: {:?}", channels);

        loop {
            select! {
                // 主动心跳
                _ = hb_interval.tick() => {
                    let ping_msg = json!({ "op": "ping", "ping": Utc::now().timestamp() }).to_string();
                    if tx.send(Message::Text(ping_msg)).await.is_err() { break; }
                }

                // 消息处理
                msg_res = ws_read.next() => {
                    match msg_res {
                        Some(Ok(msg)) => {
                            match msg {
                                Message::Text(text) => {
                                    // 这里可以增加简单的解析逻辑，如果是心跳回包则忽略，否则回调
                                    if !text.contains("\"pong\"") {
                                        if let Some(ref h) = handler { h.handle(&text).await; }
                                        if let Some(ref cb) = callback { cb(&text).await; }
                                    }
                                }
                                Message::Ping(p) => {
                                    let _ = tx.send(Message::Pong(p)).await;
                                }
                                Message::Close(_) => break,
                                _ => {}
                            }
                        }
                        Some(Err(e)) => {
                            println!("读取异常: {:?}", e);
                            break;
                        }
                        None => break,
                    }
                }

                // 监控写入任务
                _ = &mut write_task => break,
            }
        }

        write_task.abort();
        println!("连接已断开，正在尝试重连...");
        sleep(Duration::from_secs(RETRY_DELAY)).await;
    }

    Ok(())
}
