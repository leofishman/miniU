# miniU 🌌

This is a personal learning project, the goal is to create an AI assistant in a terminal-based environment that can communicate with LLMs and store the conversation history in a PostgreSQL database. It is a work in progress. I am using this project to learn Rust and LLM. (:danger:) DON'T USE THIS IN PRODUCTION, IT'S JUST FOR LEARNING!!!.

**miniU** is a lightweight, efficient, and modular LLM chat client written in Rust. It provides a terminal-based interactive interface to communicate with OpenAI-compatible APIs (like `llama.cpp` or Ollama) while maintaining a persistent conversation history in a PostgreSQL database.

## ✨ Features

- **🚀 Async & Fast**: Built on top of `tokio` and `reqwest` for non-blocking IO and high performance.
- **💾 Persistent Memory**: Automatically saves and loads chat history from a PostgreSQL database using `sqlx`.
- **🔄 Dynamic Model Switching**: List and switch between available models on the server at runtime using the `/model` command.
- **🎨 Visual Feedback**: Interactive terminal progress bars and spinners using `indicatif` to provide real-time feedback during network operations, reasoning phases, and streaming generation.
- **🧠 Reasoning Models Support**: Detects and streams model reasoning/thinking phases separately from the final response.
- **🧩 Modular Architecture**: Trait-based LLM client design (`LlmClient`), making it easy to extend for different providers.
- **📊 Usage Metrics**: Real-time tracking of token usage, generation speed (tokens/sec), and latency.

## 🛠️ Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/)
- **Runtime**: [Tokio](https://tokio.rs/)
- **HTTP Client**: [Reqwest](https://docs.rs/reqwest/)
- **Database**: [PostgreSQL](https://www.postgresql.org/) with [SQLx](https://github.com/launchbadge/sqlx)
- **Serialization**: [Serde](https://serde.rs/)
- **Logging/UX**: Custom ASCII spinners and colored terminal output.

## 🚀 Getting Started

### Prerequisites

- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install)
- **PostgreSQL**: A running instance with a database created.
- **LLM Server**: A running OpenAI-compatible server (e.g., `llama-server` from `llama.cpp`).

### Configuration

1. Create a `.env` file in the root directory:

```env
DATABASE_URL=postgres://user:password@localhost/miniu_db
LLM_BASE_URL=127.0.0.1:8080
MODEL_NAME=your-default-model-name
```

2. The application will automatically initialize the necessary tables on the first run.

### Running the App

```bash
cargo run
```

## ⌨️ Commands

During the chat session, you can use the following commands:

- `/model`: List all available models and switch the current one.
- `/exit` or `/quit` or `/q`: Close the application.
- `/download <model_name>`: (Planned) Download new models from Hugging Face.

## 🏗️ Project Structure

- `src/main.rs`: Entry point and CLI loop.
- `src/traits/llm_client.rs`: OpenAI-compatible client implementation with streaming and reasoning support.
- `src/traits/llm_scorer.rs`: LLM-based importance scoring for conversation memory management.
- `src/traits/summarizer.rs`: Conversation history summarization logic.
- `src/modules/memory/`: Database logic and conversation state management.

## 📜 License

This project is licensed under the MIT License - see the LICENSE file for details.
