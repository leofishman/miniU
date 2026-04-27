

mod traits;
mod modules;

use crate::modules::memory::Conversation;
use crate::traits::llm_client::{OpenAiClient, LlmClient};
use dotenvy::dotenv;
use sqlx::PgPool;
use std::env;
use std::io::{self, Write};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); 

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL no definida");
    let llm_url = env::var("LLM_BASE_URL").expect("LLM_BASE_URL no definida");
    let model = env::var("MODEL_NAME").expect("MODEL_NAME no definido");

    // 1. Conexión única
    let pool = PgPool::connect(&database_url).await?;

    // 2. Inicialización (solo una vez)
    crate::modules::memory::database::init_db(&pool).await?;

    let client = OpenAiClient {
        api_key: "".to_string(),
        base_url: llm_url,
        model,
    };

    // 3. Iniciar sesión (podrías usar un UUID fijo para "mismo usuario")
    let session_id = Uuid::new_v4(); 
    let limit = 10;

    // Usamos el constructor asíncrono que carga el historial
    let mut conversation = Conversation::new(client, session_id, limit, &pool).await?;

    let models: Vec<String> = conversation.client.list_models().await?;
    println!("--- miniU Chat System ---");
    println!("Session ID: {}", session_id);
    println!("Available models: {:#?}", models);
    println!("Type 'exit' or 'quit' to stop.\n");       

    let model_name = env::var("MODEL_NAME").expect("MODEL_NAME no definida");   
    println!("\nCurrent model: {}", model_name);    

    let mut input_model = String::new();
    io::stdin().read_line(&mut input_model)?; 
    let input_model = input_model.trim(); 
    if input_model != model_name {
        if !models.contains(&input_model.to_string()) {
            println!("Model not found");
            return Err("Model not found".into());
        } 
        conversation.client.model = input_model.to_string();
    }   
    

    loop {
        print!("User: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Llamada asíncrona a ask
        match conversation.ask(input.to_string(), &pool).await {
            Ok(response) => {
                println!("\nAssistant: {}\n", response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}