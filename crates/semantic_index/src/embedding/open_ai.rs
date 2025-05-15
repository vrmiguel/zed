use crate::{Embedding, EmbeddingProvider, TextToEmbed};
use anyhow::{anyhow, Result};
use credentials_provider::CredentialsProvider;
use futures::{future::BoxFuture, FutureExt};
use gpui::AsyncApp;
use http_client::HttpClient;
pub use open_ai::OpenAiEmbeddingModel;
use std::sync::Arc;

const OPENAI_CREDENTIALS_KEY: &str = "openai_api";

pub struct OpenAiEmbeddingProvider {
    client: Arc<dyn HttpClient>,
    model: OpenAiEmbeddingModel,
    api_url: String,
    credentials: Arc<dyn CredentialsProvider>,
    async_app: AsyncApp,
}

impl OpenAiEmbeddingProvider {
    pub fn new(
        client: Arc<dyn HttpClient>,
        model: OpenAiEmbeddingModel,
        api_url: String,
        credentials: Arc<dyn CredentialsProvider>,
        async_app: AsyncApp,
    ) -> Self {
        Self {
            client,
            model,
            api_url,
            credentials,
            async_app,
        }
    }

    async fn get_api_key(&self) -> Result<String> {
        let creds = self.credentials.read_credentials(OPENAI_CREDENTIALS_KEY, &self.async_app).await?;
        match creds {
            Some((_, key)) => Ok(String::from_utf8(key)?),
            None => Err(anyhow!("OpenAI API key not found in credentials store"))
        }
    }
}

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn embed<'a>(&'a self, texts: &'a [TextToEmbed<'a>]) -> BoxFuture<'a, Result<Vec<Embedding>>> {
        async move {
            let api_key = self.get_api_key().await?;
            let embed = open_ai::embed(
                self.client.as_ref(),
                &self.api_url,
                &api_key,
                self.model,
                texts.iter().map(|to_embed| to_embed.text),
            );
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
