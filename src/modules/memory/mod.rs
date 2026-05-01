use crate::traits::llm_client::{ChatMessage, OpenAiClient, LlmClient, Role};
use sqlx::PgPool;
use uuid::Uuid;
use dotenvy::dotenv;
use std::env;

pub mod database;   

pub struct Conversation  {
    pub client: OpenAiClient,
    pub session_id: Uuid,
    pub history: Vec<ChatMessage>,
    pub buffer_limit: usize,
    pub summary: String,
}

impl Conversation {
    pub async fn new(
        client: OpenAiClient, 
        session_id: Uuid, 
        limit: usize, 
        pool: &PgPool
    ) -> Result<Self, String> {
        let mut history = Self::load_history(pool, &session_id)
            .await
            .map_err(|e| e.to_string())?;
        if history.is_empty() {
            let system_msg = ChatMessage {
                role: Role::System,
                content: "Eres un asistente experto en Rust y bases de datos.".to_string(),
            };
            
            database::save_single_message(pool, &session_id, &system_msg)
                .await
                .map_err(|e| e.to_string())?;
                
            history.push(system_msg);
        }

        Ok(Self {
            client,
            session_id,
            history,
            buffer_limit: limit,
            summary: String::new(), 
        })
    }

    pub async fn ask(&mut self, question: String, pool: &PgPool) -> Result<String, String> {

        let user_msg = ChatMessage {
            role: Role::User,
            content: question,
        };
        database::save_single_message(pool, &self.session_id, &user_msg).await.map_err(|e| e.to_string())?;
        self.history.push(user_msg);

        // Always include the first message (System) and the last N messages
        let mut context_to_send = Vec::new();
        if !self.history.is_empty() {
            context_to_send.push(self.history[0].clone()); // The System Prompt
            
            let tail_start = if self.history.len() > self.buffer_limit {
                self.history.len() - self.buffer_limit
            } else {
                1 // Start after the System Prompt
            };
            
            context_to_send.extend(self.history[tail_start..].iter().cloned());
        }

        let response_text = self.client.chat(&context_to_send, pool).await?;

        let assistant_msg = ChatMessage {
            role: Role::Assistant,
            content: response_text.clone(),
        };
        database::save_single_message(pool, &self.session_id, &assistant_msg).await.map_err(|e| e.to_string())?;
        self.history.push(assistant_msg);

        Ok(response_text)
    }

    pub async fn load_history(
        pool: &PgPool, 
        session_id: &Uuid   
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        let history = database::load_history(pool, session_id).await?;
        Ok(history)
    }
    
    pub fn set_model(&mut self, new_model: String, available_models: &[String]) -> Result<(), String> {
        if !available_models.contains(&new_model) {
            dotenv().ok(); 
            self.client.model = env::var("MODEL_NAME").expect("MODEL_NAME no definido");
            // TODO: download model
            return Err(format!("Model '{}' not found. Available models: {:#?}", new_model, available_models));
        }
        self.client.model = new_model;
        println!("🔄 Model updated to: {}", self.client.model);
        Ok(())  
    }   
}