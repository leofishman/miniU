use crate::traits::llm_client::{ChatMessage, OpenAiClient, LlmClient, Role};

pub struct Conversation {
    pub client: OpenAiClient,
    pub history: Vec<ChatMessage>,
}

impl Conversation {
    pub fn ask(&mut self, question: String) -> Result<String, String> {
        let user_message = ChatMessage {
            role: Role::User,
            content: question,
        };

        self.history.push(user_message);

        let response_text = self.client.chat(&self.history)?;

        let assistant_message = ChatMessage {
            role: Role::Assistant,
            content: response_text.clone(),
        };  

        self.history.push(assistant_message);
        
        Ok(response_text)
    }
}