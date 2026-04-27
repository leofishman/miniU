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

    let http_client = reqwest::Client::new();

    let client = OpenAiClient {
        api_key: "".to_string(),
        base_url: llm_url,
        model: model.clone(),
        client: http_client,
    };

    // 3. Iniciar sesión (podrías usar un UUID fijo para "mismo usuario")
    let session_id = Uuid::new_v4(); 
    let limit = 10;

    // Usamos el constructor asíncrono que carga el historial
    let mut conversation = Conversation::new(client, session_id, limit, &pool).await?;

    ask_model(&mut conversation, model.clone(), &pool).await?;
    println!("Type 'exit' or 'quit' to stop.\n");       

    loop {
        print!("User: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.starts_with("/") {
            // expect a command
            let mut command_parts = input.split(" ");
            let command = command_parts.next().unwrap();
            let args = command_parts.collect::<Vec<&str>>();

            match command {
                "/download" => {
                    if args.len() != 1 {
                        eprintln!("Usage: /download <model_name>");
                        continue;
                    }
                    let _model_name = args[0].to_string();
                }
                "/model" => {
                    let current_model = conversation.client.model.clone();
                    let _ = ask_model(&mut conversation, current_model, &pool).await;
                }
                "/exit" => {
                    break;
                }
                "/quit" => {
                    break;
                }
                "/q" => {
                    break;
                }
                _ => {
                    eprintln!("Unknown command: {}", command);
                }
            }   
            continue;
        }

        if input.is_empty() {
            continue;
        }

        println!("🧠 Processing your request...");
        io::stdout().flush()?;

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

async fn ask_model(conversation: &mut Conversation, model: String, _pool: &PgPool) -> Result<String, Box<dyn std::error::Error>> {
    print!("📡 Fetching model list from server...");
    io::stdout().flush()?;
    
    let models: Vec<String> = conversation.client.list_models().await?;

    println!("📡 Fetched {} models from server.\n", models.len());  

    println!("\n{}", "=".repeat(100));
    println!("Available models: {:#?}", models);
    println!("{}", "=".repeat(100));
    
    println!("\nCurrent model: {}", model);    
    println!("Type a model from the list if you want to change it, otherwise just press Enter to continue with the current model.");
    
    print!("Selected model: ");
    io::stdout().flush()?;

    let mut input_model = String::new();
    io::stdin().read_line(&mut input_model)?;
    let input_model = input_model.trim();
    if input_model != model && !input_model.is_empty() {
        if let Err(e) = conversation.set_model(input_model.to_string(), &models) {
            eprintln!("Error changing model: {}", e);
        }
    }
    Ok(input_model.to_string())
    
}