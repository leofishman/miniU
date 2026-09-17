use crate::traits::llm_client::OpenAiClient;
use sqlx::PgPool;
use uuid::Uuid;

pub mod database;
pub mod state_board;

pub use state_board::StateBoard;

#[derive(Clone)]
pub struct Conversation {
    pub client: OpenAiClient,
    pub session_id: Uuid,
    pub state_board: Option<StateBoard>,
}

impl std::fmt::Debug for Conversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation")
            .field("session_id", &self.session_id)
            .field("state_board", &self.state_board)
            .finish()
    }
}

impl Conversation {
    pub async fn new(
        client: OpenAiClient,
        session_id: Uuid,
        pool: &PgPool,
    ) -> Result<Self, String> {
        let mut conv = Self {
            client,
            session_id,
            state_board: None,
        };

        conv.load_state_board(pool).await?;

        Ok(conv)
    }

    pub fn set_model(&mut self, model: String, available_models: &[String]) -> Result<(), String> {
        if !available_models.contains(&model) {
            return Err(format!("Model {} not found on the server.", model));
        }
        self.client.model = model;
        Ok(())
    }

    async fn load_state_board(&mut self, pool: &PgPool) -> Result<(), String> {
        let board_opt = database::load_state_board(pool, &self.session_id)
            .await
            .map_err(|e| e.to_string())?;
        self.state_board = board_opt;
        Ok(())
    }
}
