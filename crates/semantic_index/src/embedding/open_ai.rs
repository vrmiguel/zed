use crate::{Embedding, EmbeddingProvider, TextToEmbed};
use anyhow::{anyhow, Result};
use futures::{future::BoxFuture, FutureExt};
use http_client::HttpClient;
pub use open_ai::OpenAiEmbeddingModel;
use std::sync::Arc;

#[derive(Clone)]
struct ApiKey(String);

impl ApiKey {
    fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if key.trim().is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }
        if !key.starts_with("sk-") {
            return Err(anyhow!("Invalid API key format"));
        }
        Ok(Self(key))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct OpenAiEmbeddingProvider {
    client: Arc<dyn HttpClient>,
    model: OpenAiEmbeddingModel,
    api_url: String,
    api_key: ApiKey,
}

impl OpenAiEmbeddingProvider {
    pub fn new(
        client: Arc<dyn HttpClient>,
        model: OpenAiEmbeddingModel,
        api_url: String,
        api_key: String,
    ) -> Result<Self> {
        let api_key = ApiKey::new(api_key)?;
        Ok(Self {
            client,
            model,
            api_url,
            api_key,
        })
    }
}

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn embed<'a>(&'a self, texts: &'a [TextToEmbed<'a>]) -> BoxFuture<'a, Result<Vec<Embedding>>> {
        let embed = open_ai::embed(
            self.client.as_ref(),
            &self.api_url,
            self.api_key.as_str(),
            self.model,
            texts.iter().map(|to_embed| to_embed.text),
        );
        async move {
            let response = embed.await?;
            Ok(response
                .data
                .into_iter()
                .map(|data| Embedding::new(data.embedding))
                .collect())
        }
        .boxed()
    }

    fn batch_size(&self) -> usize {
        // From https://platform.openai.com/docs/api-reference/embeddings/create
        2048
    }
}
