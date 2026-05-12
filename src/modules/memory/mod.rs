use crate::traits::llm_client::{ChatMessage, LlmClient, OpenAiClient, Role};
use dotenvy::dotenv;
use sqlx::PgPool;
use std::env;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct Conversation {
    pub client: OpenAiClient,
    pub session_id: Uuid,
    pub history: Vec<ChatMessage>,
    pub buffer_limit: usize,
    pub summary: String,
    pub reflexion_task: Option<JoinHandle<()>>,
    pub state_board: Option<StateBoard>,
}


pub mod database;
pub mod state_board;

pub use state_board::StateBoard;


impl std::fmt::Debug for Conversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation")
            .field("session_id", &self.session_id)
            .field("history", &self.history)
            .field("buffer_limit", &self.buffer_limit)
            .field("summary", &self.summary)
            .field("reflexion_task", &self.reflexion_task)
            .finish()
    }
}

impl Conversation {
    pub async fn new(
        client: OpenAiClient,
        session_id: Uuid,
        limit: usize,
        pool: &PgPool,
    ) -> Result<Self, String> {
        let mut history = Self::load_history(pool, &session_id)
            .await
            .map_err(|e| e.to_string())?;
        if history.is_empty() {
            let system_msg = ChatMessage {
                role: Role::System,
                content: "Eres un asistente experto en Rust, llm, agentes y bases de datos."
                    .to_string(),
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
            reflexion_task: None,
            state_board: Some(StateBoard::default()),
        })
    }

    pub async fn ask(&mut self, user_input: String, pool: &PgPool) -> Result<String, String> {
        // 1. Prepare current message
        let user_msg = ChatMessage {
            role: Role::User,
            content: user_input,
        };
        
        // Save user message to DB
        database::save_single_message(pool, &self.session_id, &user_msg).await
            .map_err(|e| e.to_string())?;
        self.history.push(user_msg);

        // 2. PASS 1: Strategic Planning (Tool Call Check)
        // We inject the StateBoard as a System Prompt
        let system_prompt = self.state_board.as_ref()
            .map(|sb| sb.generate_system_prompt())
            .unwrap_or_default();

        let mut context = vec![ChatMessage {
            role: Role::System,
            content: system_prompt,
        }];
        context.extend(self.history.clone());

        let tool = crate::traits::llm_client::get_update_state_tool();
        let tool_call = self.client.chat_with_tools(&context, &[tool]).await?;

        // 3. Handle State Update if the LLM requested it
        if let Some(call) = tool_call {
            if call.function.name == "update_state" {
                let incoming_state: StateBoard = serde_json::from_str(&call.function.arguments)
                    .map_err(|e| format!("Invalid StateBoard JSON from LLM: {}", e))?;
                
                // Perform Deep Merge and update DB/Local state
                crate::modules::memory::database::update_state_board(
                    pool, 
                    &self.session_id, 
                    incoming_state, 
                    false // is_human = false
                ).await?;
                
                // Reload state from DB to ensure consistency
                self.load_state_board(pool).await?;
            }
        }

        // 4. PASS 2: Generation (Natural Language Response)
        // Refresh context with the potentially updated StateBoard
        let updated_system_prompt = self.state_board.as_ref()
            .map(|sb| sb.generate_system_prompt())
            .unwrap_or_default();

        let mut final_context = vec![ChatMessage {
            role: Role::System,
            content: updated_system_prompt,
        }];
        final_context.extend(self.history.clone());

        let response_text = self.client.chat(&final_context, pool).await?;

        // 5. Finalize turn
        let assistant_msg = ChatMessage {
            role: Role::Assistant,
            content: response_text.clone(),
        };
        
        database::save_single_message(pool, &self.session_id, &assistant_msg).await
            .map_err(|e| e.to_string())?;
        self.history.push(assistant_msg);

        Ok(response_text)
    }

    async fn load_state_board(&mut self, pool: &PgPool) -> Result<(), String> {
        let row = sqlx::query!(
            "SELECT board_json FROM session_state WHERE session_id = $1",
            self.session_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some(r) = row {
            self.state_board = Some(serde_json::from_value(r.board_json).map_err(|e| e.to_string())?);
        }
        Ok(())
    }

    pub async fn load_history(
        pool: &PgPool,
        session_id: &Uuid,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        let history = database::load_history(pool, session_id).await?;
        Ok(history)
    }

    pub fn set_model(
        &mut self,
        new_model: String,
        available_models: &[String],
    ) -> Result<(), String> {
        if !available_models.contains(&new_model) {
            dotenv().ok();
            self.client.model = env::var("MODEL_NAME").expect("MODEL_NAME no definido");
            // TODO: download model
            return Err(format!(
                "Model '{}' not found. Available models: {:#?}",
                new_model, available_models
            ));
        }
        self.client.model = new_model;
        println!("🔄 Model updated to: {}", self.client.model);
        Ok(())
    }
}
