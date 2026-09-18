use crate::traits::llm_client::OpenAiClient;
use sqlx::PgPool;
use uuid::Uuid;

pub mod database;
pub mod state_board;

pub use state_board::StateBoard;

#[derive(Clone)]
pub struct OrchestrationSession {
    pub manager_client: OpenAiClient,
    pub worker_client: OpenAiClient,
    pub session_id: Uuid,
    pub state_board: Option<StateBoard>,
}

impl std::fmt::Debug for OrchestrationSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrchestrationSession")
            .field("session_id", &self.session_id)
            .field("manager_model", &self.manager_client.model)
            .field("worker_model", &self.worker_client.model)
            .field("state_board", &self.state_board)
            .finish()
    }
}

impl OrchestrationSession {
    pub async fn new(
        manager_client: OpenAiClient,
        worker_client: OpenAiClient,
        session_id: Uuid,
        pool: &PgPool,
    ) -> Result<Self, String> {
        let mut session = Self {
            manager_client,
            worker_client,
            session_id,
            state_board: None,
        };

        session.load_state_board(pool).await?;

        Ok(session)
    }

    async fn load_state_board(&mut self, pool: &PgPool) -> Result<(), String> {
        let board_opt = database::load_state_board(pool, &self.session_id)
            .await
            .map_err(|e| e.to_string())?;
        self.state_board = board_opt;
        Ok(())
    }
}
