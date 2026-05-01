use async_trait::async_trait;
use crate::traits::llm_client::ChatMessage;

#[async_trait]
pub trait ConversationSummarizer {
    async fn update_summary(
        &self, 
        current_summary: &str, 
        recent_messages: &[ChatMessage]
    ) -> Result<String, String>;
}