use async_trait::async_trait;
use crate::traits::llm_client::ChatMessage;

#[async_trait]
pub trait ImportanceScorer {
    async fn score_message(&self, message: &ChatMessage) -> Result<u8, String>;
}