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

// --- Trait & Types ---

pub trait MessageHandler: Send + Sync {
    fn handle(&self, msg: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}

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

#[derive(Debug)]
enum WsMessage {
    Text(String),
    Pong(Vec<u8>),
    Close,
}

// --- Client Implementation ---

pub struct BitUnixWsClient {
    cmd_tx: mpsc::Sender<Cmd>,
    _handler: Arc<dyn MessageHandler>,
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
        }
    }

    pub async fn subscribe(&self, symbol: &str, interval: &str) -> Result<()> {
        let sub = Subscription {
            symbol: symbol.to_uppercase(), // 生产环境：强制大写
            interval: interval.to_string(),
        };
        self.cmd_tx.send(Cmd::Subscribe(sub)).await
            .map_err(|_| anyhow!("WS Runner 任务已停止"))
    }

    pub async fn shutdown(&self) -> Result<()> {
        let _ = self.cmd_tx.send(Cmd::Shutdown).await;
        Ok(())
    }
}

// --- Runner Implementation ---

struct WsRunner {
    wss_domain: String,
    handler: Arc<dyn MessageHandler>,
    subscriptions: HashSet<Subscription>,
    cmd_rx: mpsc::Receiver<Cmd>,
}

impl WsRunner {
    #[instrument(skip(self), fields(domain = %self.wss_domain))]
    async fn run(mut self) {
        info!("BitUnix WebSocket 后台任务启动");
        let mut retry_delay = INITIAL_RETRY_DELAY;

        loop {
            match connect_websocket(&self.wss_domain).await {
                Ok(ws_stream) => {
                    retry_delay = INITIAL_RETRY_DELAY;
                    // handle_session 返回 Some(Cmd::Shutdown) 表示需要彻底退出
                    if let Some(Cmd::Shutdown) = self.handle_session(ws_stream).await {
                        info!("收到关闭指令，彻底退出任务循环");
                        break;
                    }
                    warn!("Session 断开，1秒后尝试重连...");
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    error!(error = ?e, "连接失败，将在 {} 秒后重试", retry_delay);
                    sleep(Duration::from_secs(retry_delay)).await;
                    retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                }
            }
        }
    }

    async fn handle_session(&mut self, ws_stream: WsStream) -> Option<Cmd> {
        let (mut write_stream, mut read_stream) = ws_stream.split();
        let (msg_tx, mut msg_rx) = mpsc::channel::<WsMessage>(CHANNEL_BUFFER);
        let (write_abort_tx, mut write_abort_rx) = oneshot::channel();

        // 关键：连接状态管理
        let mut is_authenticated = false;

        // 1. 写入协程
        let write_handle = tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                let res = match msg {
                    WsMessage::Text(text) => write_stream.send(Message::Text(text)).await,
                    WsMessage::Pong(data) => write_stream.send(Message::Pong(data)).await,
                    WsMessage::Close => {
                        let _ = write_stream.close().await;
                        break;
                    }
                };
                if let Err(e) = res {
                    error!(error = ?e, "WebSocket 写入失败");
                    break;
                }
            }
            let _ = write_abort_tx.send(());
        });

        let mut heartbeat_timer = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        heartbeat_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut return_reason = None;

        loop {
            select! {
                // A. 处理命令（订阅/取消/关闭）
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(Cmd::Subscribe(sub)) => {
                            let is_new = self.subscriptions.insert(sub.clone());
                            if is_new && is_authenticated {
                                let msg = create_sub_json(&sub, "subscribe");
                                let _ = msg_tx.send(WsMessage::Text(msg)).await;
                            }
                        }
                        Some(Cmd::Unsubscribe(sub)) => {
                            if self.subscriptions.remove(&sub) && is_authenticated {
                                let msg = create_sub_json(&sub, "unsubscribe");
                                let _ = msg_tx.send(WsMessage::Text(msg)).await;
                            }
                        }
                        Some(Cmd::Shutdown) => {
                            let _ = msg_tx.send(WsMessage::Close).await;
                            return_reason = Some(Cmd::Shutdown);
                            break;
                        }
                        None => break, // Sender 已被释放
                    }
                }

                // B. 定时心跳 (Ping)
                _ = heartbeat_timer.tick() => {
                    let ping = json!({ "op": "ping", "ts": Utc::now().timestamp_millis() }).to_string();
                    if msg_tx.send(WsMessage::Text(ping)).await.is_err() { break; }
                }

                // C. 处理接收的消息
                msg = read_stream.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // 优先处理系统级消息
                            if text.contains(r#""op":"connect""#) {
                                if text.contains(r#""result":true"#) {
                                    info!("WebSocket 认证成功，同步 {} 个订阅任务", self.subscriptions.len());
                                    is_authenticated = true;
                                    // 批量补发断线前/等待中的订阅
                                    for sub in &self.subscriptions {
                                        let _ = msg_tx.send(WsMessage::Text(create_sub_json(sub, "subscribe"))).await;
                                    }
                                } else {
                                    error!("WebSocket 认证失败消息: {}", text);
                                }
                                continue;
                            }

                            // 处理 Ping 并返回 Pong
                            if text.contains(r#""op":"ping""#) {
                                let pong = text.replace("ping", "pong");
                                let _ = msg_tx.send(WsMessage::Text(pong)).await;
                                continue;
                            }

                            // 业务数据处理
                            let h = self.handler.clone();
                            tokio::spawn(async move {
                                h.handle(&text).await;
                            });
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = msg_tx.send(WsMessage::Pong(data)).await;
                        }
                        Some(Ok(Message::Close(frame))) => {
                            warn!(?frame, "服务器主动断开连接");
                            break;
                        }
                        Some(Err(e)) => {
                            error!(error = ?e, "读取数据流错误");
                            break;
                        }
                        None => {
                            warn!("连接流已关闭 (EOF)");
                            break;
                        }
                        _ => {}
                    }
                }

                // D. 写入任务崩溃监控
                _ = &mut write_abort_rx => {
                    error!("写入协程意外终止");
                    break;
                }
            }
        }

        // 清理 Session
        let _ = write_handle.abort();
        return_reason
    }
}

// --- Utils ---

async fn connect_websocket(wss_domain: &str) -> Result<WsStream> {
    let url = format!("wss://{}/public/", wss_domain);
    debug!("尝试连接: {}", url);
    let (ws, _) = tokio::time::timeout(Duration::from_secs(10), connect_async(&url))
        .await
        .map_err(|_| anyhow!("连接超时"))??;

    info!("TCP 握手完成，等待 Connect 确认...");
    Ok(ws)
}

fn create_sub_json(sub: &Subscription, op: &str) -> String {
    // 根据 BitUnix 官方格式，args 是一个对象数组
    json!({
        "op": op,
        "args": [{
            "symbol": sub.symbol,
            "ch": sub.interval,
        }]
    }).to_string()
}
