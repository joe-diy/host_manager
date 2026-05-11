//! Typed HTTP client for the Host Manager REST API.

use anyhow::{Context, Result};
use reqwest::{header, Client};
use serde::de::DeserializeOwned;
use std::time::Duration;

pub struct ApiClient {
    http: Client,
    base_url: String,
    pub output_format: String,
}

impl ApiClient {
    pub fn new(base_url: String, api_key: Option<String>, output: &str) -> Result<Self> {
        let mut headers = header::HeaderMap::new();

        // Prefer API key header; fall back to stored token cookie.
        if let Some(key) = api_key {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {key}"))
                    .context("invalid API key format")?,
            );
        } else if let Some(token) = crate::auth::load_token()? {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {token}"))
                    .context("invalid stored token")?,
            );
        }

        let http = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http,
            base_url,
            output_format: output.to_string(),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        self.http
            .get(&url)
            .send()
            .await
            .context("request failed")?
            .error_for_status()
            .context("server error")?
            .json::<T>()
            .await
            .context("response deserialization failed")
    }

    pub async fn post<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        self.http
            .post(&url)
            .json(body)
            .send()
            .await
            .context("request failed")?
            .error_for_status()
            .context("server error")?
            .json::<T>()
            .await
            .context("response deserialization failed")
    }
}
