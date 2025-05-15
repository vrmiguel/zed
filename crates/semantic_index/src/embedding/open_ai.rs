use crate::{Embedding, EmbeddingProvider, TextToEmbed};
use anyhow::{Result, Context};
use futures::{future::BoxFuture, FutureExt};
use http_client::HttpClient;
pub use open_ai::OpenAiEmbeddingModel;
use std::{env, sync::Arc};

const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";

pub struct OpenAiEmbeddingProvider {
    client: Arc<dyn HttpClient>,
    model: OpenAiEmbeddingModel,
    api_url: String,
    api_key: Arc<String>,
}

impl OpenAiEmbeddingProvider {
    pub fn new(
        client: Arc<dyn HttpClient>,
        model: OpenAiEmbeddingModel,
        api_url: String,
    ) -> Result<Self> {
        let api_key = env::var(OPENAI_API_KEY_ENV)
            .with_context(|| format!("Missing environment variable: {}", OPENAI_API_KEY_ENV))?;
        
        if api_key.trim().is_empty() {
            anyhow::bail!("OpenAI API key cannot be empty");
        }
        
        Ok(Self {
            client,
            model,
            api_url,
            api_key: Arc::new(api_key),
        })
    }
}

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn embed<'a>(&'a self, texts: &'a [TextToEmbed<'a>]) -> BoxFuture<'a, Result<Vec<Embedding>>> {
        let embed = open_ai::embed(
            self.client.as_ref(),
            &self.api_url,
            self.api_key.as_ref(),
            self.model,
            texts.iter().map(|to_embed| to_embed.text),
        );
        async move {
            let response = embed.await.with_context(|| "Failed to generate embeddings with OpenAI API")?;
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
