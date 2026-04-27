use crate::traits::llm_client::{ChatMessage, OpenAiClient, LlmClient, Role};
use sqlx::PgPool;
use uuid::Uuid;

pub mod database;   

pub struct Conversation  {
    pub client: OpenAiClient,
    pub session_id: Uuid,
    pub history: Vec<ChatMessage>,
    pub limit: usize,
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
            
            // Guardamos el primer mensaje en la DB inmediatamente
            database::save_single_message(pool, &session_id, &system_msg)
                .await
                .map_err(|e| e.to_string())?;
                
            history.push(system_msg);
        }

        Ok(Self {
            client,
            session_id,
            history,
            limit,
        })
    }

    pub async fn ask(&mut self, question: String, pool: &PgPool) -> Result<String, String> {

        let user_msg = ChatMessage {
            role: Role::User,
            content: question,
        };

        database::save_single_message(pool, &self.session_id, &user_msg)
            .await
            .map_err(|e| e.to_string())?;
        
        self.history.push(user_msg);

        let response_text = self.client.chat(&self.history, pool).await?;
        let assistant_msg = ChatMessage {
            role: Role::Assistant,
            content: response_text.clone(),
        };

        database::save_single_message(pool, &self.session_id, &assistant_msg)
            .await
            .map_err(|e| e.to_string())?;

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
}