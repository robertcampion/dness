use anyhow::{Context as _, Result};
use serde::de::DeserializeOwned;

pub trait ClientErrorWrapper {
    type Response;

    async fn send_err(self, context: &str) -> Result<Self::Response>;
    async fn send_text(self, context: &str) -> Result<String>;
    async fn send_json<T: DeserializeOwned>(self, context: &str) -> Result<T>;
}

impl ClientErrorWrapper for reqwest::RequestBuilder {
    type Response = reqwest::Response;

    async fn send_err(self, context: &str) -> Result<reqwest::Response> {
        let (client, request) = self.build_split();
        let request = request?;
        let url = request.url().as_str().to_owned();
        client
            .execute(request)
            .await
            .with_context(|| {
                format!("unable to send http request for {context}: url attempted: {url}",)
            })?
            .error_for_status()
            .with_context(|| {
                format!("received bad http response for {context}: url attempted: {url}")
            })
    }

    async fn send_text(self, context: &str) -> Result<String> {
        let response = self.send_err(context).await?;
        let url = response.url().as_str().to_owned();
        response.text().await.with_context(|| {
            format!("unable to deserialize response for {context}: url attempted: {url}",)
        })
    }
    async fn send_json<T: DeserializeOwned>(self, context: &str) -> Result<T> {
        let response = self.send_err(context).await?;
        let url = response.url().as_str().to_owned();
        response.json().await.with_context(|| {
            format!("unable to deserialize response for {context}: url attempted: {url}",)
        })
    }
}
