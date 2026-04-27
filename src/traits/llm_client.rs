use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use async_trait::async_trait;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;

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

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, messages: &[ChatMessage], _pool: &PgPool) -> Result<String, String> {
        let request_body = ChatRequest {
            model: &self.model,
            messages,
            stream: true,
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

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.map_err(|e| e.to_string())?;
            return Err(format!("Server error {}: {}", status, body));
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap());
        pb.set_message("Processing...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let mut stream = response.bytes_stream();
        let mut full_content = String::new();
        let mut first_token = true;
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.starts_with("data:") {
                    let data = line["data:".len()..].trim();
                    if data == "[DONE]" {
                        break;
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        // Aggressively search for content in the JSON structure
                        let mut found_content = None;
                        
                        if let Some(choices) = json["choices"].as_array() {
                            for choice in choices {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                        found_content = Some(content);
                                        break;
                                    }
                                    if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                                        found_content = Some(reasoning);
                                        break;
                                    }
                                } else if let Some(text) = choice.get("text").and_then(|v| v.as_str()) {
                                    found_content = Some(text);
                                    break;
                                }
                            }
                        }
                        
                        if found_content.is_none() {
                            if let Some(content) = json.get("content").and_then(|v| v.as_str()) {
                                found_content = Some(content);
                            }
                        }

                        if let Some(content) = found_content {
                            if first_token {
                                pb.finish_and_clear();
                                print!("Assistant: ");
                                std::io::stdout().flush().unwrap();
                                first_token = false;
                            }
                            print!("{}", content);
                            std::io::stdout().flush().unwrap();
                            full_content.push_str(content);
                        }
                        
                        // Handle usage and timings if present in the final stream chunk
                        if let Some(usage_val) = json.get("usage") {
                            if let Ok(usage) = serde_json::from_value::<Usage>(usage_val.clone()) {
                                eprintln!("\n\n--- LLM Usage ---");
                                eprintln!("Tokens used: {} (Prompt: {}, Completion: {})", 
                                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens);
                            }
                        }
                        if let Some(timings_val) = json.get("timings") {
                            if let Ok(timings) = serde_json::from_value::<Timings>(timings_val.clone()) {
                                eprintln!("Time: {:.2}s | Speed: {:.2} t/s", 
                                    timings.predicted_ms / 1000.0, timings.predicted_per_second);
                                eprintln!("-----------------\n");
                            }
                        }
                    }
                }
            }
        }

        // Process any remaining content in the buffer
        if !buffer.is_empty() {
            let line = buffer.trim();
            if line.starts_with("data:") {
                let data = line["data:".len()..].trim();
                if data != "[DONE]" && !data.is_empty() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            if first_token {
                                pb.finish_and_clear();
                                print!("Assistant: ");
                                std::io::stdout().flush().unwrap();
                                first_token = false;
                            }
                            print!("{}", content);
                            std::io::stdout().flush().unwrap();
                            full_content.push_str(content);
                        }
                    }
                }
            }
        }

        if first_token {
            pb.finish_and_clear();
        } else {
            println!(); // New line after streaming
        }

        if full_content.is_empty() {
            return Err("Received empty response from server. Check if the model is loaded and the server is responding correctly.".to_string());
        }

        Ok(full_content)
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
