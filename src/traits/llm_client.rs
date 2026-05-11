use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use async_trait::async_trait;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::io::Write;
use std::fs::OpenOptions;

// remove after getting uuid from conversation
use crate::Uuid;

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

#[derive(Clone)]
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

enum Delta {
    Content(String),
    Reasoning(String),
    None,
}

#[allow(dead_code)]
pub struct MemoryStatus {
    pub history_count: usize,
    pub buffer_limit: usize,
    pub summary_length: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
    pub r#type: String, // Always "function"
    pub function: FunctionDefinition,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string
}

impl OpenAiClient {
    fn get_url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        if base.starts_with("http") {
            format!("{}/{}", base, path)
        } else {
            format!("http://{}/{}", base, path)
        }
    }
    
    fn extract_delta<'a>(&self, json: &'a serde_json::Value) -> Delta { 
        if let Some(choices) = json["choices"].as_array() {
            if let Some(delta) = choices.get(0).and_then(|c| c.get("delta")) {
                if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                    return Delta::Reasoning(reasoning.to_string());
                }
                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                    return Delta::Content(content.to_string());
                }
            }
        }
        Delta::None
    }

    fn log_reasoning(&self, content: &str, session_id: &Uuid) {
        if content.is_empty() { return; }
        // Crear la carpeta logs si no existe
        let _ = std::fs::create_dir_all("logs");
        // TODO: Add logging filename by session_id from Conversation struct 
        let log_file = format!("logs/{}.log", session_id);
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file) {
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(file, "\n--- SESSION {} [{}] ---\n{}\n", self.model, timestamp, content);
            }
    }

    fn log_metrics(&self, json: &serde_json::Value) {
        if let Some(usage_val) = json.get("usage") {
            if let Ok(usage) = serde_json::from_value::<Usage>(usage_val.clone()) {
                eprintln!("\n\n--- LLM Usage ---");
                eprintln!("Tokens used: {} (Prompt: {}, Completion: {})", 
                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens);
                eprintln!("-----------------");
            }
        }
        if let Some(timings_val) = json.get("timings") {
            if let Ok(timings) = serde_json::from_value::<Timings>(timings_val.clone()) {
                eprintln!("\n\n--- Performance ---");
                eprintln!("\t\tTime: {:.2}s \t|\t Speed: {:.2} t/s", 
                    timings.predicted_ms / 1000.0, timings.predicted_per_second);
                eprintln!("-----------------\n");
            }
        }
    }

    #[allow(dead_code)]
    pub async fn chat_raw(&self, messages: &[ChatMessage]) -> Result<String, String> {
        let response = self.call_completions(messages, false).await?;

        if !response.status().is_success() {
            return Err(format!("Server error: {}", response.status()));
        }

        // Para tareas raw, parseamos el JSON completo de la respuesta de OpenAI
        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        // Extraemos el contenido del primer choice
        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No se encontró contenido en la respuesta raw".to_string())
    }

    pub async fn chat_with_tools(
        &self, 
        messages: &[ChatMessage], 
        tools: &[Tool]
    ) -> Result<Option<ToolCall>, String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "stream": false // Tool calling is more stable without streaming in Pass 1
        });

        let response = self.client
            .post(&self.get_url("/v1/chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        // Check if the model generated a tool call
        if let Some(tool_calls) = json["choices"][0]["message"]["tool_calls"].as_array() {
            if let Some(first_call) = tool_calls.get(0) {
                let call: ToolCall = serde_json::from_value(first_call.clone())
                    .map_err(|e| e.to_string())?;
                return Ok(Some(call));
            }
        }

        Ok(None)
    }

    pub fn get_update_state_tool() -> Tool {
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "update_state".to_string(),
                description: "Update the StateBoard layers (L1, L2, L3, L4) to maintain context.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "l1_immediate": { "type": "object" },
                        "l2_task": { "type": "object" },
                        "l3_semantic": { "type": "object" },
                        "l4_history": { "type": "array", "items": { "type": "object" } }
                    }
                }),
            },
        }
    }
    
    #[allow(dead_code)]
    async fn call_completions(
        &self, 
        messages: &[ChatMessage], 
        stream: bool
    ) -> Result<reqwest::Response, String> {
        let request_body = ChatRequest {
            model: &self.model,
            messages,
            stream,
        };

        self.client.post(&self.get_url("/v1/chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| e.to_string())
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, messages: &[ChatMessage], _pool: &PgPool) -> Result<String, String> {
        let request_body = ChatRequest {
            model: &self.model,
            messages,
            stream: true,
        };
        let response = self.client.post(&self.get_url("/v1/chat/completions"))
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
        
        let multi = MultiProgress::new();

        // NIVEL 1: Estado General
        let pb_main = multi.add(ProgressBar::new_spinner());
        pb_main.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap());
        pb_main.set_message("Conectando con el cerebro...");
        pb_main.enable_steady_tick(std::time::Duration::from_millis(100));

        // NIVEL 2: Razonamiento
        let pb_reasoning = multi.add(ProgressBar::new_spinner());
        pb_reasoning.set_style(ProgressStyle::default_spinner()
            .template("\x1b[2m{spinner:.blue} [Pensando]: {msg}\x1b[0m") 
            .unwrap());

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut stream_done = false;
        
        let mut reasoning_full = String::new();
        let mut response_full = String::new();
        let mut first_content_token = true;

        while let Some(chunk) = stream.next().await {
            if stream_done { break; }
            let chunk = chunk.map_err(|e| e.to_string())?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.starts_with("data:") {
                    let data = &line["data:".len()..].trim();
                    if *data == "[DONE]" { 
                        stream_done = true;
                        break; 
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        match self.extract_delta(&json) {
                            Delta::Reasoning(r) => {
                                pb_main.set_message("Generando razonamiento...");
                                // El replace es para que no rompa la línea del spinner
                                pb_reasoning.set_message(r.clone().replace('\n', " "));
                                reasoning_full.push_str(&r);
                            }
                            Delta::Content(c) => {
                                if first_content_token {
                                    pb_main.finish_and_clear();
                                    pb_reasoning.finish_and_clear();
                                    
                                    // Log de razonamiento (ahora que sabemos que empezó la respuesta)
                                    self.log_reasoning(&reasoning_full, &uuid::Uuid::nil()); 
                                    
                                    print!("Assistant: ");
                                    std::io::stdout().flush().unwrap();
                                    first_content_token = false;
                                }
                                print!("{}", c);
                                std::io::stdout().flush().unwrap();
                                response_full.push_str(&c);
                            }
                            Delta::None => {}
                        }
                        self.log_metrics(&json);
                    }
                }
            }
        }

        // Limpieza final de barras si no se limpiaron antes
        if first_content_token {
            pb_main.finish_and_clear();
            pb_reasoning.finish_and_clear();
        }
        println!(); 

        if response_full.is_empty() {
            return Err("El modelo no devolvió ninguna respuesta.".to_string());
        }

        Ok(response_full)
    }

    async fn list_models(&self) -> Result<Vec<String>, String> {
        let response = self.client.get(&self.get_url("/v1/models"))
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
