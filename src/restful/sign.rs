use anyhow::Result;
use hex; // 引入 hex 编码
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sha256::digest; // 引入 SHA256 和 Digest trait
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
        let nonce = nanoid::nanoid!(32);

        let query_str = Self::build_query_string(parameters);
        let body_str = "";
        let pre_sign = format!(
            "{}{}{}{}{}",
            nonce, timestamp, self.api_key, query_str, body_str
        );
        let digest = Self::sha256_hex(&pre_sign);
        let sign_input = format!("{}{}", digest, self.secret_key);
        let sign = Self::sha256_hex(&sign_input);

        let url = self.build_full_url(request_path, parameters);

        if self.debug {
            println!("FIRST_SIGN:{}", pre_sign.clone());
            println!("[GET] URL: {}", url);
            println!("[GET] Params: {:?}", parameters);
            println!("SECOND SIGN: {:?}", sign_input.clone());
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
        let nonce = nanoid::nanoid!(8);

        let query_str = Self::build_query_string(query_params);
        let compact_body = Self::compact_json(body)?;
        let pre_sign = format!(
            "{}{}{}{}{}",
            nonce, timestamp, self.api_key, query_str, compact_body
        );

        let digest = Self::sha256_hex(&pre_sign);
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
        let mut result = String::new();
        for (k, v) in params {
            result.push_str(k);
            result.push_str(v);
        }
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
        Ok(serde_json::to_string(body)?.replace(' ', ""))
    }

    /// 计算输入字符串的 SHA-256 十六进制字符串
    fn sha256_hex(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)

        //digest(input)
    }

    pub fn get_timestamp(&self) -> String {
        chrono::Utc::now().timestamp_millis().to_string()
    }
}
