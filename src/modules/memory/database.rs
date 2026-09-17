use super::StateBoard;
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

/// Initialize the database and ensure the tables and indexes exist.
pub async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    // 1. Sessions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id UUID PRIMARY KEY,
            title TEXT DEFAULT 'New Session', 
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 2. Session State (StateBoard) with versioning for Optimistic Locking
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS session_state (
            id SERIAL PRIMARY KEY,
            session_id UUID NOT NULL UNIQUE REFERENCES sessions(id) ON DELETE CASCADE,
            board_json JSONB NOT NULL,
            version BIGINT DEFAULT 1,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn load_state_board(
    pool: &PgPool,
    session_id: &Uuid,
) -> Result<Option<StateBoard>, sqlx::Error> {
    let row = sqlx::query("SELECT board_json FROM session_state WHERE session_id = $1")
        .bind(session_id)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        let board_json: serde_json::Value = row.get("board_json");
        let board: StateBoard =
            serde_json::from_value(board_json).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        Ok(Some(board))
    } else {
        Ok(None)
    }
}

pub async fn init_session_state(
    pool: &PgPool,
    session_id: &Uuid,
    initial_state: &StateBoard,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Insert session if not exists
    sqlx::query("INSERT INTO sessions (id, title) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING")
        .bind(session_id)
        .bind("Zero-Shot Orchestration")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    // Insert session_state if not exists
    let state_json = serde_json::to_value(initial_state).map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO session_state (session_id, board_json) VALUES ($1, $2) ON CONFLICT (session_id) DO NOTHING")
        .bind(session_id)
        .bind(state_json)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn update_state_board(
    pool: &PgPool,
    session_id: &Uuid,
    incoming_data: StateBoard,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Fetch current state
    let row = sqlx::query(
        "SELECT board_json, version FROM session_state WHERE session_id = $1 FOR UPDATE",
    )
    .bind(session_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(row) = row {
        let board_json: serde_json::Value = row.get("board_json");
        let mut current_board: StateBoard =
            serde_json::from_value(board_json).map_err(|e| e.to_string())?;

        // 2. Fusionar
        current_board.merge(incoming_data);

        // 3. Guardar e incrementar versión
        sqlx::query(
            "UPDATE session_state 
             SET board_json = $1, version = version + 1, updated_at = NOW() 
             WHERE session_id = $2",
        )
        .bind(serde_json::to_value(current_board).unwrap())
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    } else {
        // Fallback: If for some reason the row doesn't exist, we insert the incoming data directly
        let state_json = serde_json::to_value(&incoming_data).map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO session_state (session_id, board_json) VALUES ($1, $2)")
            .bind(session_id)
            .bind(state_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}
