use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, oneshot},
    time::{interval, sleep, Duration, MissedTickBehavior},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, instrument, warn};

// 假设这是你定义的 Handler trait
pub trait MessageHandler: Send + Sync {
    fn handle(&self, msg: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}

// --- 常量定义 ---
const HEARTBEAT_INTERVAL: u64 = 15;
const INITIAL_RETRY_DELAY: u64 = 5;
const MAX_RETRY_DELAY: u64 = 60;
const CHANNEL_BUFFER: usize = 1000;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Subscription {
    symbol: String,
    interval: String,
}

#[derive(Debug)]
enum Cmd {
    Subscribe(Subscription),
    Unsubscribe(Subscription),
    Shutdown,
}

// ==========================================
// 缺少的部分在这里：
// ==========================================
#[derive(Debug)]
enum WsMessage {
    Text(String),
    Pong(Vec<u8>),
    Close,
}
// ==========================================

pub struct BitUnixWsClient {
    cmd_tx: mpsc::Sender<Cmd>,
    _handler: Arc<dyn MessageHandler>,
    wss_domain: String,
}

impl BitUnixWsClient {
    pub fn new(wss_domain: &str, handler: Arc<dyn MessageHandler>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(CHANNEL_BUFFER);
        let domain = wss_domain.to_string();
        let handler_clone = handler.clone();

        tokio::spawn(async move {
            let runner = WsRunner {
                wss_domain: domain,
                handler: handler_clone,
                subscriptions: HashSet::new(),
                cmd_rx,
            };
            runner.run().await;
        });

        Self {
            cmd_tx,
            _handler: handler,
            wss_domain: wss_domain.to_string(),
        }
    }

    pub async fn subscribe(&self, symbol: &str, interval: &str) -> Result<()> {
        let sub = Subscription {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
        };
        self.cmd_tx.send(Cmd::Subscribe(sub)).await
            .map_err(|_| anyhow!("后台任务已停止"))
    }

    pub async fn unsubscribe(&self, symbol: &str, interval: &str) -> Result<()> {
        let sub = Subscription {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
        };
        self.cmd_tx.send(Cmd::Unsubscribe(sub)).await
            .map_err(|_| anyhow!("后台任务已停止"))
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.cmd_tx.send(Cmd::Shutdown).await
            .map_err(|_| anyhow!("后台任务已停止"))
    }
}

struct WsRunner {
    wss_domain: String,
    handler: Arc<dyn MessageHandler>,
    subscriptions: HashSet<Subscription>,
    cmd_rx: mpsc::Receiver<Cmd>,
}


impl WsRunner {
    // 1. 优化连接函数：确保路径正确
    async fn connect_websocket(wss_domain: &str) -> Result<WsStream> {
        // 修正点：BitUnix 路径通常包含 /ws/。 如果你传入的是 "ws.bitunix.com"，则拼接成如下：
        let url = format!("wss://{}/ws/public", wss_domain);
        info!(%url, "正在尝试建立 WebSocket 连接...");

        let (ws, _) = tokio::time::timeout(
            Duration::from_secs(10),
            connect_async(&url)
        ).await.map_err(|_| anyhow!("连接超时"))??;

        info!("WebSocket 连接成功建立");
        Ok(ws)
    }

    async fn handle_session(&mut self, ws_stream: WsStream) -> Option<Cmd> {
        let (mut write_stream, mut read_stream) = ws_stream.split();
        let (msg_tx, mut msg_rx) = mpsc::channel::<WsMessage>(CHANNEL_BUFFER);
        let (write_abort_tx, mut write_abort_rx) = oneshot::channel();

        // --- 写入任务 ---
        let write_handle = tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                let res = match msg {
                    WsMessage::Text(text) => {
                        debug!("发送消息: {}", text);
                        write_stream.send(Message::Text(text)).await
                    },
                    WsMessage::Pong(data) => write_stream.send(Message::Pong(data)).await,
                    WsMessage::Close => {
                        let _ = write_stream.close().await;
                        break;
                    }
                };
                if let Err(e) = res {
                    error!("WS 发送失败: {:?}", e);
                    break;
                }
            }
            let _ = write_abort_tx.send(());
        });

        // --- 自动恢复订阅 ---
        for sub in self.subscriptions.iter() {
            let msg = create_sub_json(sub, "subscribe");
            let _ = msg_tx.send(WsMessage::Text(msg)).await;
        }

        let mut heartbeat_timer = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        heartbeat_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut return_reason: Option<Cmd> = None;

        loop {
            select! {
                // 处理外部订阅命令
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(Cmd::Subscribe(sub)) => {
                            if self.subscriptions.insert(sub.clone()) {
                                let msg = create_sub_json(&sub, "subscribe");
                                let _ = msg_tx.send(WsMessage::Text(msg)).await;
                            }
                        }
                        Some(Cmd::Unsubscribe(sub)) => {
                            if self.subscriptions.remove(&sub) {
                                let msg = create_sub_json(&sub, "unsubscribe");
                                let _ = msg_tx.send(WsMessage::Text(msg)).await;
                            }
                        }
                        Some(Cmd::Shutdown) => {
                            let _ = msg_tx.send(WsMessage::Close).await;
                            return_reason = Some(Cmd::Shutdown);
                            break;
                        }
                        None => break,
                    }
                }

                // 主动心跳
                _ = heartbeat_timer.tick() => {
                    let ping = json!({ "op": "ping", "ping": Utc::now().timestamp() }).to_string();
                    if msg_tx.send(WsMessage::Text(ping)).await.is_err() { break; }
                }

                // 处理接收到的消息 (核心修复区)
                msg_res = read_stream.next() => {
                    match msg_res {
                        Some(Ok(Message::Text(text))) => {
                            // 打印原始消息以便调试
                            info!("收到原始消息: {}", text);

                            // 解析 JSON 判断类型
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                // 1. 响应服务器的心跳
                                if val["op"] == "ping" {
                                    let mut pong_val = val.clone();
                                    pong_val["op"] = json!("pong");
                                    // 根据文档，有些需要把接收到的 ping 值原样返回
                                    let _ = msg_tx.send(WsMessage::Text(pong_val.to_string())).await;
                                    continue;
                                }

                                // 2. 处理订阅/取消订阅的结果消息
                                if val["op"] == "subscribe" || val["op"] == "unsubscribe" {
                                    info!("操作确认: {} -> {:?}", val["op"], val["args"]);
                                    // 订阅结果也可以传给 handler，或者在这里拦截
                                }
                            }

                            // 3. 业务数据，交给 Handler
                            let h = self.handler.clone();
                            tokio::spawn(async move {
                                h.handle(&text).await;
                            });
                        }
                        Some(Ok(Message::Ping(d))) => { let _ = msg_tx.send(WsMessage::Pong(d)).await; }
                        Some(Ok(Message::Close(_))) => { warn!("服务端连接关闭"); break; }
                        Some(Err(e)) => { error!("读取错误: {:?}", e); break; }
                        None => { warn!("连接已断开 (None)"); break; }
                        _ => {}
                    }
                }

                // 监控写入任务
                _ = &mut write_abort_rx => {
                    error!("写入后台任务崩溃");
                    break;
                }
            }
        }

        write_handle.abort();
        return_reason
    }
}


async fn connect_websocket(wss_domain: &str) -> Result<WsStream> {
    let url = format!("wss://{}/public/", wss_domain);
    debug!(%url, "正在连接...");
    let connect_future = connect_async(&url);
    let (ws, _) = tokio::time::timeout(Duration::from_secs(10), connect_future)
        .await
        .map_err(|_| anyhow!("连接超时"))??;

    info!("WebSocket 连接已建立");
    Ok(ws)
}

fn create_sub_json(sub: &Subscription, op: &str) -> String {
    json!({
        "op": op,
        "args": [{
            "symbol": sub.symbol,
            "ch": sub.interval,
        }]
    }).to_string()
}
