use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, Mutex},
    time::{interval, sleep, Duration, MissedTickBehavior},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, instrument, warn};

// 假设这是你定义的 Handler trait
use super::types::MessageHandler;

// --- 常量定义 ---
const HEARTBEAT_INTERVAL: u64 = 20;
const INITIAL_RETRY_DELAY: u64 = 5;
const MAX_RETRY_DELAY: u64 = 60;
const CHANNEL_BUFFER: usize = 1000; // 内部命令通道大小

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// 订阅请求的数据结构，用于去重和重连
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Subscription {
    symbol: String,
    interval: String,
}

/// 发送给后台任务的控制命令
#[derive(Debug)]
enum Cmd {
    Subscribe(Subscription),
    Unsubscribe(Subscription),
    Shutdown,
}

/// BitUnix WebSocket 客户端管理器
pub struct BitUnixWsClient {
    cmd_tx: mpsc::Sender<Cmd>,
    handler: Arc<dyn MessageHandler>,
    wss_domain: String,
}

impl BitUnixWsClient {
    /// 创建并启动客户端
    pub fn new(wss_domain: &str, handler: Arc<dyn MessageHandler>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(CHANNEL_BUFFER);
        let domain = wss_domain.to_string();
        let handler_clone = handler.clone();

        // 启动后台守护任务
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
            handler,
            wss_domain: wss_domain.to_string(),
        }
    }

    /// 动态添加订阅
    pub async fn subscribe(&self, symbol: &str, interval: &str) -> Result<()> {
        let sub = Subscription {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
        };
        self.cmd_tx.send(Cmd::Subscribe(sub)).await
            .map_err(|_| anyhow!("后台任务已停止"))
    }

    /// 动态取消订阅
    pub async fn unsubscribe(&self, symbol: &str, interval: &str) -> Result<()> {
        let sub = Subscription {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
        };
        self.cmd_tx.send(Cmd::Unsubscribe(sub)).await
            .map_err(|_| anyhow!("后台任务已停止"))
    }

    /// 关闭连接
    pub async fn shutdown(&self) -> Result<()> {
        self.cmd_tx.send(Cmd::Shutdown).await
            .map_err(|_| anyhow!("后台任务已停止"))
    }
}

// --- 后台运行逻辑 ---

struct WsRunner {
    wss_domain: String,
    handler: Arc<dyn MessageHandler>,
    // 核心：在内存中记录当前应该订阅的所有频道
    subscriptions: HashSet<Subscription>,
    cmd_rx: mpsc::Receiver<Cmd>,
}

impl WsRunner {
    #[instrument(skip(self), fields(domain = %self.wss_domain))]
    async fn run(mut self) {
        info!("BitUnix WebSocket 后台任务启动");
        let mut retry_delay = INITIAL_RETRY_DELAY;

        loop {
            // 1. 尝试连接
            let ws_stream = match connect_websocket(&self.wss_domain).await {
                Ok(s) => {
                    retry_delay = INITIAL_RETRY_DELAY; // 连接成功，重置退避
                    s
                }
                Err(e) => {
                    error!(error = ?e, "连接失败，将在 {}秒 后重试", retry_delay);
                    sleep(Duration::from_secs(retry_delay)).await;
                    retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                    continue;
                }
            };

            // 2. 运行会话（直到断开）
            let disconnect_reason = self.handle_session(ws_stream).await;

            // 3. 检查是否是主动关闭
            if let Some(Cmd::Shutdown) = disconnect_reason {
                info!("收到关闭指令，退出 WebSocket 任务");
                break;
            }

            warn!("连接断开，准备重连...");
            // 这里不需要 sleep，因为 handle_session 内部断开通常意味着网络问题或EOF，
            // 立即重连或在外层循环有 connect 的重试逻辑。
            // 为了防止死循环轰炸，可以在这里加个小延迟
            sleep(Duration::from_secs(1)).await;
        }
    }

    /// 处理单个 WebSocket 会话
    /// 返回值：如果是因收到内部命令退出，返回该命令；否则返回 None (表示网络断开)
    async fn handle_session(&mut self, ws_stream: WsStream) -> Option<Cmd> {
        let (mut write, mut read) = ws_stream.split();

        // 关键点：重连成功后，立即重新发送内存中已有的所有订阅
        if !self.subscriptions.is_empty() {
            info!("检测到 {} 个活跃订阅，正在恢复...", self.subscriptions.len());
            for sub in self.subscriptions.iter() {
                if let Err(e) = send_subscribe_msg(&mut write, sub, "subscribe").await {
                    error!(error = ?e, symbol = %sub.symbol, "恢复订阅失败");
                }
            }
        }

        let mut heartbeat_timer = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        heartbeat_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            select! {
                // A. 处理外部控制命令 (订阅/取消/关闭)
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(Cmd::Subscribe(sub)) => {
                            if self.subscriptions.contains(&sub) {
                                debug!(symbol = %sub.symbol, "忽略重复订阅");
                            } else {
                                info!(symbol = %sub.symbol, interval = %sub.interval, "添加新订阅");
                                if let Err(e) = send_subscribe_msg(&mut write, &sub, "subscribe").await {
                                    error!(error = ?e, "发送订阅指令失败");
                                    // 即使发送失败，也要决定是否存入 set？通常建议存入，以便重连时重试
                                }
                                self.subscriptions.insert(sub);
                            }
                        }
                        Some(Cmd::Unsubscribe(sub)) => {
                            if self.subscriptions.remove(&sub) {
                                info!(symbol = %sub.symbol, "取消订阅");
                                // 尽最大努力发送 unsubscribe，失败也不影响本地移除
                                let _ = send_subscribe_msg(&mut write, &sub, "unsubscribe").await;
                            }
                        }
                        Some(Cmd::Shutdown) => return Some(Cmd::Shutdown),
                        None => return Some(Cmd::Shutdown), // 发送端都丢弃了，任务结束
                    }
                }

                // B. 心跳保活
                _ = heartbeat_timer.tick() => {
                    let ping = json!({ "op": "ping", "ping": Utc::now().timestamp() }).to_string();
                    if let Err(e) = write.send(Message::Text(ping)).await {
                        error!(error = ?e, "心跳发送失败，判定连接断开");
                        return None;
                    }
                }

                // C. 处理接收到的 WebSocket 消息
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                             // 关键优化：异步处理消息，不要阻塞主 select 循环
                             // 如果 handler 计算量大，建议使用 tokio::spawn
                             let h = self.handler.clone();
                             tokio::spawn(async move {
                                 h.handle(&text).await;
                             });
                        }
                        Some(Ok(Message::Pong(_))) => { /* 心跳响应 */ }
                        Some(Ok(Message::Close(_))) => {
                            warn!("服务端发送 Close 帧");
                            return None;
                        }
                        Some(Err(e)) => {
                            error!(error = ?e, "WebSocket 读取异常");
                            return None;
                        }
                        None => {
                            warn!("WebSocket EOF (流结束)");
                            return None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// --- 辅助函数 ---

async fn connect_websocket(wss_domain: &str) -> Result<WsStream> {
    let url = format!("wss://{}/public/", wss_domain);
    debug!(%url, "正在连接...");
    let (ws, _) = connect_async(&url).await?;
    info!("WebSocket 连接已建立");
    Ok(ws)
}

async fn send_subscribe_msg<S>(write: &mut S, sub: &Subscription, op: &str) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    let msg = json!({
        "op": op,
        "args": [{
            "symbol": sub.symbol,
            "ch": sub.interval,
        }]
    }).to_string();

    write.send(Message::Text(msg)).await
        .map_err(|e| anyhow!("写消息失败: {:?}", e))
}
