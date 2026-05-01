use serde_json::json;
use crate::traits::llm_client::{ChatMessage, Role};
use crate::traits::scorer::ImportanceScorer;
use crate::traits::llm_client::OpenAiClient;
use crate::traits::llm_client::LlmClient;
use async_trait::async_trait;

#[async_trait]
impl ImportanceScorer for OpenAiClient {
    async fn score_message(&self, message: &ChatMessage, _pool: &sqlx::PgPool) -> Result<u8, String> {
        let system_prompt = "Tu tarea es evaluar la 'Importancia Informativa' de un mensaje en una conversación técnica sobre Rust y SQL. \
                             Responde ÚNICAMENTE con un objeto JSON: {\"score\": [1-10]}. \
                             1: Saludos, confirmaciones cortas, charlatanería. \
                             10: Fragmentos de código nuevos, comandos SQL, decisiones de arquitectura, nombres de entidades críticos.";

        let messages = vec![
            ChatMessage { role: Role::System, content: system_prompt.to_string() },
            ChatMessage { role: Role::User, content: format!("Evalúa este mensaje: '{}'", message.content) },
        ];

        // TODO: Tendríamos que adaptar chat() o crear una chat_json() en OpenAiClient.
        let raw_response = self.chat(&messages, _pool).await?; // Asumiendo stream:false temporalmente

        // 3. Parseo y Validación del JSON
        let json_score: serde_json::Value = serde_json::from_str(&raw_response)
            .map_err(|e| format!("Error parseando puntuación: {}", e))?;

        let score = json_score["score"].as_u64()
            .ok_or("El formato JSON de puntuación es inválido".to_string())? as u8;

        // Validamos que esté en rango
        if score < 1 || score > 10 {
            return Err("Puntuación fuera de rango (1-10)".to_string());
        }

        Ok(score)
    }
}