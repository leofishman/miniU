use crate::traits::cursor::{CursorBusy, SpinnerType, Cursor};
use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use async_trait::async_trait;
use futures_util::StreamExt;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Serialize, Debug)]
pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [ChatMessage],
    pub stream: bool,
}


#[async_trait]
pub trait LlmClient {
    async fn chat(&self, messages: &[ChatMessage], _pool: &PgPool) -> Result<String, String>;
    async fn list_models(&self) -> Result<Vec<String>, String>;
}

pub struct OpenAiClient {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct Timings {
    predicted_ms: f64,
    predicted_per_second: f64,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
    timings: Option<Timings>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: String,
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, messages: &[ChatMessage], _pool: &PgPool) -> Result<String, String> {
        let request_body = ChatRequest {
            model: &self.model,
            messages,
            stream: false,
        };

        let base = self.base_url.trim_end_matches('/');
        let url = if base.starts_with("http") {
            format!("{}/v1/chat/completions", base)
        } else {
            format!("http://{}/v1/chat/completions", base)
        };

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        let mut cursor = CursorBusy::new(SpinnerType::Moon);
        let mut total_bytes = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            total_bytes += chunk.len();
            cursor.tick(&format!("Processing... {} bytes", total_bytes));
            body.extend_from_slice(&chunk);
        }

        let chat_completion: ChatResponse = serde_json::from_slice(&body).map_err(|e| {
            format!("JSON Error: {} | Raw body: {}", e, String::from_utf8_lossy(&body))
        })?;

        // Logging info
        if let Some(usage) = &chat_completion.usage {
            eprintln!("\n--- LLM Usage ---");
            eprintln!("Tokens used: {} (Prompt: {}, Completion: {})", 
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens);
        }
        if let Some(timings) = &chat_completion.timings {
            eprintln!("Time: {:.2}s | Speed: {:.2} t/s", 
                timings.predicted_ms / 1000.0, timings.predicted_per_second);
            eprintln!("-----------------\n");
        }

        if let Some(choice) = chat_completion.choices.into_iter().next() {
            return Ok(choice.message.content);
        }
        
        Err("No chat completion choice found in response".to_string())
    }

    async fn list_models(&self) -> Result<Vec<String>, String> {
        let base = self.base_url.trim_end_matches('/');
        let url = if base.starts_with("http") {
            format!("{}/v1/models", base)
        } else {
            format!("http://{}/v1/models", base)
        };

        let response = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelData>,
        }
        #[derive(Deserialize)]
        struct ModelData {
            id: String,
        }

        let models_data: ModelsResponse = response.json().await.map_err(|e| e.to_string())?;

        Ok(models_data.data.into_iter().map(|m| m.id).collect())
    }
}
