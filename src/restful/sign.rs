use anyhow::Result;
use rand::{distributions::Alphanumeric, Rng};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::client::BitUnixClient;

impl BitUnixClient {
    pub async fn get<T>(
        &self,
        request_path: &str,
        parameters: &BTreeMap<String, String>,
    ) -> Result<T>
    where
        T: DeserializeOwned + std::fmt::Debug,
    {
        let timestamp = self.get_timestamp();
        let nonce = Self::generate_nonce();

        let query_str = Self::build_query_string(parameters);
        let body_str = "";
        let pre_sign = format!(
            "{}{}{}{}{}",
            nonce, timestamp, self.api_key, query_str, body_str
        );
        let digest = Self::sha256_hex(&pre_sign);
        let sign = Self::sha256_hex(&format!("{}{}", digest, self.secret_key));

        let url = self.build_full_url(request_path, parameters);

        if self.debug {
            println!("FIRST_SIGN:{}", pre_sign.clone());
            println!("[GET] URL: {}", url);
            println!("[GET] Params: {:?}", parameters);
            println!("[GET] Sign: {}", sign);
        }

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("api-key", &self.api_key)
            .header("nonce", &nonce)
            .header("time", &timestamp)
            .header("sign", sign)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .text()
            .await?;

        if self.debug {
            println!("[GET] Response: {:#?}", resp);
        }

        Ok(serde_json::from_str::<T>(&resp)?)
    }

    pub async fn post<T>(
        &self,
        request_path: &str,
        query_params: &BTreeMap<String, String>,
        body: &Value,
    ) -> Result<T>
    where
        T: DeserializeOwned + std::fmt::Debug,
    {
        let timestamp = self.get_timestamp();
        let nonce = Self::generate_nonce();

        let query_str = Self::build_query_string(query_params);
        let compact_body = Self::compact_json(body)?;
        let first_digest_input = format!(
            "{}{}{}{}{}",
            nonce, timestamp, self.api_key, query_str, compact_body
        );

        let digest = Self::sha256_hex(&first_digest_input);
        let sign_input = format!("{}{}", digest, self.secret_key);
        let sign = Self::sha256_hex(&sign_input);

        let url = self.build_full_url(request_path, query_params);

        if self.debug {
            println!("[POST] URL: {}", url);
            println!("[POST] Body: {}", compact_body);
            println!("[POST] Sign: {}", sign);
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .header("Nonce", &nonce)
            .header("Timestamp", &timestamp)
            .header("Sign", sign)
            .header("Content-Type", "application/json")
            .body(compact_body)
            .send()
            .await?
            .json::<T>()
            .await?;

        if self.debug {
            println!("[POST] Response: {:#?}", resp);
        }

        Ok(resp)
    }

    /// 构建 query 参数的签名字符串（key+value 按照 ASCII 排序）
    fn build_query_string(params: &BTreeMap<String, String>) -> String {
        let result = params
            .iter()
            .map(|(k, v)| format!("{}{}", k, v))
            .collect::<Vec<_>>()
            .join("");
        result
    }

    /// 构建完整 URL（含 query 参数）
    fn build_full_url(&self, path: &str, params: &BTreeMap<String, String>) -> String {
        let domain = self.domain.trim_end_matches('/');
        let path = path.trim_start_matches('/');

        if params.is_empty() {
            format!("{}/{}", domain, path)
        } else {
            let query_string = params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}/{}?{}", domain, path, query_string)
        }
    }

    /// 将 JSON 对象转为紧凑格式（无空格）
    fn compact_json(body: &Value) -> Result<String> {
        Ok(body.to_string().replace(' ', ""))
    }

    /// 计算输入字符串的 SHA-256 十六进制字符串
    fn sha256_hex(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn get_timestamp(&self) -> String {
        chrono::Utc::now().timestamp_millis().to_string()
    }

    pub fn generate_nonce() -> String {
        let nonce: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        nonce
    }
}
