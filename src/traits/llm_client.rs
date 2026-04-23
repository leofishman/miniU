use serde::{Serialize, Deserialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use sqlx::PgPool;
use async_trait::async_trait;

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
    async fn chat(&self, messages: &[ChatMessage], pool: &PgPool) -> Result<String, String>;
}

pub struct OpenAiClient {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
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
    async fn chat(&self, messages: &[ChatMessage], pool: &PgPool) -> Result<String, String> {
        let request_body = ChatRequest {
            model: &self.model,
            messages,
            stream: false,
        };

        let body_json = serde_json::to_vec(&request_body).map_err(|e| e.to_string())?;

        let mut stream = TcpStream::connect(&self.base_url).map_err(|e| e.to_string())?;
        
        stream.write_all(b"POST /v1/chat/completions HTTP/1.1\r\n").map_err(|e| e.to_string())?;
        stream.write_all(format!("Host: {}\r\n", self.base_url).as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(b"Content-Type: application/json\r\n").map_err(|e| e.to_string())?;
        stream.write_all(format!("Authorization: Bearer {}\r\n", self.api_key).as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(format!("Content-Length: {}\r\n", body_json.len()).as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(b"Connection: close\r\n").map_err(|e| e.to_string())?;
        stream.write_all(b"\r\n").map_err(|e| e.to_string())?;
        stream.write_all(&body_json).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        let mut response = String::new();
        stream.read_to_string(&mut response).map_err(|e| e.to_string())?;
        
        if let Some(pos) = response.find("\r\n\r\n") {
            let headers = &response[..pos];
            let body = &response[pos + 4..];

            let mut final_body = String::new();
            if headers.to_lowercase().contains("transfer-encoding: chunked") {
                let mut current = body;
                while !current.is_empty() {
                    if let Some(chunk_pos) = current.find("\r\n") {
                        let size_str = &current[..chunk_pos].trim();
                        if let Ok(size) = usize::from_str_radix(size_str, 16) {
                            if size == 0 { break; }
                            let chunk_start = chunk_pos + 2;
                            if current.len() >= chunk_start + size {
                                final_body.push_str(&current[chunk_start..chunk_start + size]);
                                current = &current[chunk_start + size + 2..]; // skip chunk and trailing \r\n
                            } else {
                                break;
                            }
                        } else {
                            final_body = body.to_string();
                            break;
                        }
                    } else {
                        break;
                    }
                }
            } else {
                final_body = body.to_string();
            }

            let json_body = final_body.trim();
            let chat_completion: ChatResponse = serde_json::from_str(json_body).map_err(|e| {
                format!("JSON Error: {} | Raw body: {}", e, json_body)
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
        }
        
        Err("No se encontró el cuerpo de la respuesta o formato inválido".to_string())
    }
}
