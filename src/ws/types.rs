use async_trait::async_trait;

#[async_trait]
pub trait MessageHandler: Send + Sync + 'static {
    async fn handle(&self, msg: &str);
}

pub type MessageCallback = Box<
    dyn Fn(&str) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync
        + 'static,
>;
