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

    #[instrument(skip(self), fields(domain = %self.wss_domain))]

    async fn run(mut self) {

        info!("BitUnix WebSocket 后台任务启动");

        let mut retry_delay = INITIAL_RETRY_DELAY;



        loop {

            let ws_stream = match connect_websocket(&self.wss_domain).await {

                Ok(s) => {

                    retry_delay = INITIAL_RETRY_DELAY;

                    s

                }

                Err(e) => {

                    error!(error = ?e, "连接失败，将在 {}秒 后重试", retry_delay);

                    sleep(Duration::from_secs(retry_delay)).await;

                    retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);

                    continue;

                }

            };



            if let Some(Cmd::Shutdown) = self.handle_session(ws_stream).await {

                info!("收到关闭指令，退出 WebSocket 任务");

                break;

            }



            warn!("连接断开，准备重连...");

            sleep(Duration::from_secs(1)).await;

        }

    }



    async fn handle_session(&mut self, ws_stream: WsStream) -> Option<Cmd> {

        let (mut write_stream, mut read_stream) = ws_stream.split();

        let (msg_tx, mut msg_rx) = mpsc::channel::<WsMessage>(CHANNEL_BUFFER);



        // 注意这里的 mut

        let (write_abort_tx, mut write_abort_rx) = oneshot::channel();



        // --- 写入任务 ---

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



        // --- 恢复订阅 ---

        if !self.subscriptions.is_empty() {

            info!("正在恢复 {} 个活跃订阅...", self.subscriptions.len());

            for sub in self.subscriptions.iter() {

                let msg = create_sub_json(sub, "subscribe");

                // 如果发送失败，说明写入任务刚启动就挂了，循环中会检测到

                if let Err(_) = msg_tx.send(WsMessage::Text(msg)).await { break; }

            }

        }



        let mut heartbeat_timer = interval(Duration::from_secs(HEARTBEAT_INTERVAL));

        heartbeat_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);



        let return_reason: Option<Cmd>;



        loop {

            select! {

                // A. 外部命令

                cmd = self.cmd_rx.recv() => {

                    match cmd {

                        Some(Cmd::Subscribe(sub)) => {

                            if !self.subscriptions.contains(&sub) {

                                let msg = create_sub_json(&sub, "subscribe");

                                info!("Sending subscription message: {:#?}", msg);

                                let _ = msg_tx.send(WsMessage::Text(msg)).await;

                                self.subscriptions.insert(sub);

                            }

                        }

                        Some(Cmd::Unsubscribe(sub)) => {

                            if self.subscriptions.remove(&sub) {

                                let msg = create_sub_json(&sub, "unsubscribe");

                                info!("Sending unsubscription message: {:#?}", msg);

                                let _ = msg_tx.send(WsMessage::Text(msg)).await;

                            }

                        }

                        Some(Cmd::Shutdown) => {

                            let _ = msg_tx.send(WsMessage::Close).await;

                            return_reason = Some(Cmd::Shutdown);

                            break;

                        }

                        None => {

                            return_reason = Some(Cmd::Shutdown);

                            break;

                        }

                    }

                }



                // B. 心跳

                _ = heartbeat_timer.tick() => {

                    let ping = json!({ "op": "ping", "ping": Utc::now().timestamp_millis() }).to_string();

                    if msg_tx.send(WsMessage::Text(ping)).await.is_err() {

                        return_reason = None;

                        break;

                    }

                }



                // C. 读取消息

                msg = read_stream.next() => {

                    match msg {

                        Some(Ok(Message::Text(text))) => {

                            // 优先处理 JSON 格式的 Ping

                            if text.contains(r#""op":"ping""#) || text.contains(r#""op": "ping""#) {

                                let pong = text.replace("ping", "pong");

                                let _ = msg_tx.send(WsMessage::Text(pong)).await;

                            } else {

                                let h = self.handler.clone();

                                let t_clone = text.clone();

                                tokio::spawn(async move {

                                    h.handle(&t_clone).await;

                                });

                            }

                        }

                        Some(Ok(Message::Pong(_))) => {}

                        Some(Ok(Message::Ping(data))) => {

                            let _ = msg_tx.send(WsMessage::Pong(data)).await;

                        }

                        Some(Ok(Message::Close(_))) => {

                            warn!("服务端发送 Close 帧");

                            return_reason = None;

                            break;

                        }

                        Some(Err(e)) => {

                            error!(error = ?e, "WebSocket 读取异常");

                            return_reason = None;

                            break;

                        }

                        None => {

                            warn!("WebSocket EOF");

                            return_reason = None;

                            break;

                        }

                        _ => {}

                    }

                }



                // D. 写入任务监控 (注意这里的 &mut)

                _ = &mut write_abort_rx => {

                    error!("写入任务意外退出，重置连接");

                    return_reason = None;

                    break;

                }

            }

        }



        let _ = write_handle.await;

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
            "channel": sub.interval,
        }]

    }).to_string()

}
