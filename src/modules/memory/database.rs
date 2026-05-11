use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use crate::traits::llm_client::{ChatMessage, Role};

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
        "#
    )
    .execute(pool)
    .await?;

    // 2. Chat history table with 'archived' for the Forget Gate
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_history (
            id SERIAL PRIMARY KEY,
            session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            archived BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        );
        "#
    )
    .execute(pool)
    .await?;

    // 3. Partial Index for active context (Optimizes L1/L2 retrieval)
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_chat_active_context 
        ON chat_history (session_id) WHERE (archived = FALSE);
        "#
    )
    .execute(pool)
    .await?;

    // 4. Session State (StateBoard) with versioning for Optimistic Locking
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS session_state (
            id SERIAL PRIMARY KEY,
            session_id UUID NOT NULL UNIQUE REFERENCES sessions(id) ON DELETE CASCADE,
            board_json JSONB NOT NULL,
            version BIGINT DEFAULT 1,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        );
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Load chat history for a given session, ordered by creation time.
pub async fn load_history(
    pool: &PgPool,
    session_id: &Uuid,
) -> Result<Vec<ChatMessage>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT role, content FROM chat_history WHERE session_id = $1  AND archived = FALSE ORDER BY created_at ASC"
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    let history = rows
        .into_iter()
        .map(|row| {
            let role_str: String = row.get("role");
            let content: String = row.get("content");
            let role = match role_str.to_lowercase().as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                _ => Role::User,
            };
            ChatMessage {
                role,
                content,
            }
        })
        .collect();

    Ok(history)
}

/// Save a single message to the database.
pub async fn save_single_message(
    pool: &PgPool,
    session_id: &Uuid,
    message: &ChatMessage,
) -> Result<(), sqlx::Error> {
    let role_str = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
    };

    sqlx::query(
        "INSERT INTO chat_history (session_id, role, content) VALUES ($1, $2, $3)"
    )
    .bind(session_id)
    .bind(role_str)
    .bind(&message.content)
    .execute(pool)
    .await?;
    Ok(())
}