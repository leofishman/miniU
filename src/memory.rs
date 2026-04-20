use crate::traits::llm_client::{ChatMessage, OpenAiClient, LlmClient, Role};
use sqlx::PgPool;
use uuid::Uuid;

// pub struct Conversation {
//     pub client: OpenAiClient,
//     pub session_id: Uuid,
//     pub history: Vec<ChatMessage>,
//     pub limit: usize,
//     pub fn new(client: OpenAiClient, session_id: Uuid, limit: usize) -> Self {
//         let mut history = Self::load_history(pool, &session_id).await?;
//         if history.len() < 1 {
//             let context = ChatMessage {
//                 role: Role::System,
//                 content: "You are a helpful assistant.".to_string(),
//             };
//             Self::save_history(pool, &session_id, context).await?;
//         } elif history.len() > limit {
//             let new_history = history.iter().skip(1).cloned().collect();
//             Self::trim_history(pool, &session_id, limit).await?;
//         }

//         Self {
//             client,
//             session_id,
//             history,
//             limit,
//         }
//     }
// }

impl Conversation {
    pub async fn new(
        client: OpenAiClient, 
        session_id: Uuid, 
        limit: usize, 
        pool: &PgPool
    ) -> Result<Self, String> {
        // 1. Intentamos cargar el historial existente
        let mut history = Self::load_history(pool, &session_id)
            .await
            .map_err(|e| e.to_string())?;

        // 2. Si es una sesión nueva (vacía), inicializamos el System Prompt
        if history.is_empty() {
            let system_msg = ChatMessage {
                role: Role::System,
                content: "Eres un asistente experto en Rust y bases de datos.".to_string(),
            };
            
            // Guardamos el primer mensaje en la DB inmediatamente
            Self::save_single_message(pool, &session_id, &system_msg)
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
}

impl Conversation {
    pub async fn ask(&mut self, question: String, pool: &PgPool) -> Result<String, String> {
        let user_message = ChatMessage {
            role: Role::User,
            content: question,
        };

        // self.history.push(user_message);
        let mut context = Self::load_history(pool, &self.session_id).await?;
        
        context.push(user_message); 

        let response_text = self.client.chat(&context, pool).await?;

        let assistant_message = ChatMessage {
            role: Role::Assistant,
            content: response_text.clone(),
        };  

        self.history.push(user_message);
        self.history.push(assistant_message);

        Ok(response_text)
    }

    pub async fn load_history(
        pool: &PgPool, 
        session_id: &Uuid
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        let rows = sqlx::query!(
            "SELECT role, content FROM chat_history 
            WHERE session_id = $1 
            ORDER BY created_at ASC",
            session_id
        )
        .fetch_all(pool)
        .await?;

        let history = rows.into_iter().map(|row| ChatMessage {
            role: row.role,
            content: row.content,
        }).collect();

        Ok(history)
    }

    async fn save_single_message(
        pool: &PgPool,
        session_id: &Uuid,
        message: &ChatMessage,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO chat_history (session_id, role, content) VALUES ($1, $2, $3)",
            session_id,
            message.role,
            message.content
        )
        .execute(pool)
        .await?;
        Ok(())
    }   

    // pub async fn save_history(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
    //     let mut tx = pool.begin().await?;

    //     for message in &self.history {
    //         sqlx::query!(
    //             "INSERT INTO chat_history (session_id, role, content) VALUES ($1, $2, $3)",
    //             &self.session_id,
    //             message.role,
    //             message.content
    //         )
    //         .execute(&mut tx)
    //         .await?;
    //     }

    //     tx.commit().await?;

    //     Ok(())
    // }

    async fn trim_history(
        pool: &PgPool, 
        session_id: &Uuid,
        limit: usize
        ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        for message in &self.history {
            sqlx::query!(
                "INSERT INTO chat_history (session_id, role, content) VALUES ($1, $2, $3)",
                &self.session_id,
                message.role,
                message.content
            )
            .execute(&mut tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}