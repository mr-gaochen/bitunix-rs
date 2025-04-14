use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct BitUnixClient {
    pub debug: bool,
    pub testnet: bool,
    pub api_key: String,
    pub secret_key: String,
    pub domain: String,
}

impl BitUnixClient {
    pub fn new(
        debug: bool,
        testnet: bool,
        api_key: impl Into<String>,
        secret_key: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        BitUnixClient {
            debug: debug,
            testnet: testnet,
            api_key: api_key.into(),
            secret_key: secret_key.into(),
            domain: domain.into(),
        }
    }
}
