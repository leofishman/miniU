mod traits;

use crate::traits::llm_client::{LlmClient, OpenAiClient, ChatMessage};

const ENCRYPTION_KEY: &str = "";
const MODEL: &str = "ggml-org/gemma-4-26B-A4B-it-GGUF:Q4_K_M";
const BASE_URL: &str = "localhost:8080";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    // Llamamos a la funcion chat de la struct OpenAiClient implementada en Traits::LlmClient
    let client = OpenAiClient {
        api_key: ENCRYPTION_KEY.to_string(),
        base_url: BASE_URL.to_string(),
        model: MODEL.to_string(),
    };

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "Hello, how are you?".to_string(),
    }];

    let response = client.chat(messages).map_err(|e| e)?;
    println!("{}", response);


    // // 1. Abrimos el "tubo"
    // let mut stream = TcpStream::connect("127.0.0.1:8080")?;

    // // 2. Preparamos la "carta" (Petición HTTP)
    // // El doble \r\n\r\n al final es crucial: le dice al servidor "aquí termina mi pedido"
    // let request = "GET /v1/models HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";

    // // 3. Convertimos a bytes y enviamos
    // stream.write_all(request.as_bytes())?;
    
    // println!("Solicitud enviada...");
    // // 4. Creamos un espacio temporal (buffer) de 1024 bytes
    // let mut buffer = [0; 1024]; 

    // // 5. Leemos del "tubo" y guardamos cuántos bytes recibimos
    // let bytes_leidos = stream.read(&mut buffer)?;

    // println!("Leímos {} bytes del servidor.", bytes_leidos);

    // // Usamos solo los bytes que el servidor realmente escribió
    // let texto_recibido = String::from_utf8_lossy(&buffer[..bytes_leidos]);

    // println!("El servidor dice: {}", texto_recibido);

    // // Definimos un mensaje básico (como el estándar de OpenAI)
    // pub struct ChatMessage {
    //     pub role: String,    // "user", "assistant" o "system"
    //     pub content: String,
    // }

    // // Nuestro contrato para cualquier motor de IA
    // pub trait LlmClient {
    //     // Esta función toma una lista de mensajes y devuelve la respuesta del modelo
    //     fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, String>;
    // }



    // let mut response = String::new();
    // stream.read_to_string(&mut response)?;
    // println!("Respuesta: {}", response);

    Ok(())
}