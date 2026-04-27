use serde::{Serialize, Deserialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
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
    async fn list_models(&self) -> Result<Vec<String>, String>;
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

        let mut stream = TcpStream::connect(&self.base_url).await.map_err(|e| e.to_string())?;
        
        let request = format!(
            "POST /v1/chat/completions HTTP/1.1\r\n\
Host: {}\r\n\
Content-Type: application/json\r\n\
Authorization: Bearer {}\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
\r\n",
            self.base_url, self.api_key, body_json.len()
        );

        stream.write_all(request.as_bytes()).await.map_err(|e| e.to_string())?;
        stream.write_all(&body_json).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await.map_err(|e| e.to_string())?;
        
        let final_body = parse_http_response(&response)?;
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
        
        Err("No chat completion choice found in response".to_string())
    }

    async fn list_models(&self) -> Result<Vec<String>, String> {
        let mut stream = TcpStream::connect(&self.base_url).await.map_err(|e| e.to_string())?;

        let request = format!(
            "GET /v1/models HTTP/1.1\r\n\
Host: {}\r\n\
Authorization: Bearer {}\r\n\
Connection: close\r\n\
\r\n",
            self.base_url, self.api_key
        );

        stream.write_all(request.as_bytes()).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await.map_err(|e| e.to_string())?;

        let final_body = parse_http_response(&response)?;
        let json_body = final_body.trim();

        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelData>,
        }
        #[derive(Deserialize)]
        struct ModelData {
            id: String,
        }

        let models_data: ModelsResponse = serde_json::from_str(json_body).map_err(|e| {
            format!("JSON Error: {} | Raw body: {}", e, json_body)
        })?;

        Ok(models_data.data.into_iter().map(|m| m.id).collect())
    }
}

fn parse_http_response(response: &str) -> Result<String, String> {
    let (headers, body) = response.split_once("\r\n\r\n")
        .ok_or_else(|| "No se encontró el cuerpo de la respuesta o formato inválido".to_string())?;

    let is_chunked = headers.lines()
        .any(|l| {
            let l = l.to_lowercase();
            l.starts_with("transfer-encoding:") && l.contains("chunked")
        });

    if is_chunked {
        let mut final_body = String::new();
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
                    return Err("Invalid chunk size format".to_string());
                }
            } else {
                break;
            }
        }
        Ok(final_body)
    } else {
        Ok(body.to_string())
    }
}
