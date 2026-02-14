use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
const MAX_RETRY_DELAY: u64 = 60;

// 定义订阅参数结构，兼容有无 symbol 的情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub ch: String,
    pub symbol: Option<String>,
}

// --- 工具函数 ---

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

// --- 协议逻辑 ---

/// 登录逻辑 (文档：op: login)
async fn login(tx: &mpsc::Sender<Message>, api_key: &str, secret_key: &str) -> Result<()> {
    let timestamp = Utc::now().timestamp_millis(); // 文档示例中使用的是毫秒级别长整数
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

    tx.send(Message::Text(login_msg)).await?;
    Ok(())
}

/// 批量订阅逻辑 (文档：op: subscribe)
async fn subscribe(tx: &mpsc::Sender<Message>, subs: &[Subscription]) -> Result<()> {
    if subs.is_empty() { return Ok(()); }

    let subscribe_msg = json!({
        "op": "subscribe",
        "args": subs
    }).to_string();

    println!("发送订阅请求: {}", subscribe_msg);
    tx.send(Message::Text(subscribe_msg)).await?;
    Ok(())
}

// --- 对外公开接口 ---

pub async fn run_with_handler(
    wss_domain: &str,
    api_key: &str,
    secret_key: &str,
    subs: Vec<Subscription>,
    handler: Arc<dyn MessageHandler>,
) -> Result<()> {
    run_internal(wss_domain, api_key, secret_key, subs, Some(handler), None).await
}

pub async fn run_with_callback(
    wss_domain: &str,
    api_key: &str,
    secret_key: &str,
    subs: Vec<Subscription>,
    callback: MessageCallback,
) -> Result<()> {
    run_internal(wss_domain, api_key, secret_key, subs, None, Some(callback)).await
}

// --- 核心运行逻辑 ---

async fn run_internal(
    wss_domain: &str,
    api_key: &str,
    secret_key: &str,
    subs: Vec<Subscription>,
    handler: Option<Arc<dyn MessageHandler>>,
    callback: Option<MessageCallback>,
) -> Result<()> {
    let ws_url = format!("wss://{}/private/", wss_domain);
    let mut retry_delay = RETRY_DELAY;

    loop {
        println!("尝试连接 BitUnix WebSocket: {}", ws_url);

        let connect_res = connect_async(&ws_url).await;
        let (ws_stream, _) = match connect_res {
            Ok(v) => v,
            Err(e) => {
                println!("连接失败: {:?}, {}秒后重试", e, retry_delay);
                sleep(Duration::from_secs(retry_delay)).await;
                retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                continue;
            }
        };

        retry_delay = RETRY_DELAY; // 连接成功，重置重试延迟
        let (mut ws_write, mut ws_read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<Message>(300); // 增加容量以应对高频行情

        // 1. 登录
        if let Err(e) = login(&tx, api_key, secret_key).await {
            println!("登录指令发送失败: {:?}", e);
            continue;
        }

        // 2. 订阅
        if let Err(e) = subscribe(&tx, &subs).await {
            println!("订阅指令发送失败: {:?}", e);
            continue;
        }

        // 3. 独立写入任务
        let mut write_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = ws_write.send(msg).await {
                    println!("WebSocket 写入错误: {:?}", e);
                    break;
                }
            }
        });

        // 4. 心跳定时器 (文档：op: ping, ping: seconds)
        let mut hb_interval = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        hb_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        println!("BitUnix WebSocket 已启动并成功订阅频道");

        // 5. 主事件循环
        loop {
            select! {
                // 定时主动发送 Ping
                _ = hb_interval.tick() => {
                    let ping_body = json!({
                        "op": "ping",
                        "ping": Utc::now().timestamp()
                    }).to_string();
                    if tx.send(Message::Text(ping_body)).await.is_err() { break; }
                }

                // 处理接收的消息
                msg_res = ws_read.next() => {
                    match msg_res {
                        Some(Ok(msg)) => {
                            match msg {
                                Message::Text(text) => {
                                    // 预处理消息判断是否是服务器发来的 ping
                                    if let Ok(val) = serde_json::from_str::<Value>(&text) {
                                        if val["op"] == "ping" {
                                            // 如果收到服务器的 ping (文档显示服务器会返回包含 pong 的结构)
                                            // 且文档显示响应包含 pong/ping，这里作为心跳响应，保持链路。
                                            // 如果服务端是主动发 ping，客户端应回应。
                                            continue;
                                        }
                                    }

                                    // 业务回调
                                    if let Some(ref h) = handler { h.handle(&text).await; }
                                    if let Some(ref cb) = callback { cb(&text).await; }
                                }
                                Message::Ping(p) => {
                                    // 响应标准 WS 协议层 Ping
                                    let _ = tx.send(Message::Pong(p)).await;
                                }
                                Message::Close(_) => {
                                    println!("收到服务器 Close 帧");
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Some(Err(e)) => {
                            println!("读取消息异常: {:?}", e);
                            break;
                        }
                        None => {
                            println!("连接被远端关闭");
                            break;
                        }
                    }
                }

                // 监控写入任务状态
                _ = &mut write_task => {
                    println!("写入线程已退出");
                    break;
                }
            }
        }

        // 释放资源并尝试重连
        write_task.abort();
        println!("正在重连...");
        sleep(Duration::from_secs(RETRY_DELAY)).await;
    }
}
