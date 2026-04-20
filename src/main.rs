mod traits;
mod memory;

use crate::traits::llm_client::OpenAiClient;
use crate::memory::Conversation;
use std::io::{self, Write};

const ENCRYPTION_KEY: &str = "";
const MODEL: &str = "ggml-org/gemma-4-26B-A4B-it-GGUF:Q4_K_M";
const BASE_URL: &str = "localhost:8080";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let client = OpenAiClient {
        api_key: ENCRYPTION_KEY.to_string(),
        base_url: BASE_URL.to_string(),
        model: MODEL.to_string(),
    };

    let mut conversation = Conversation {
        client,
        history: Vec::new(),
    };

    println!("--- miniU Chat System ---");
    println!("Type 'exit' or 'quit' to stop.\n");

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

        match conversation.ask(input.to_string()) {
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