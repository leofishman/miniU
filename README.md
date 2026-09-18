# miniU 🌌

This is an experimental learning project focused on implementing a **Zero-Shot Self-Orchestrator** in Rust based on the architecture outlined in the research paper *"Zero-Shot Self-Orchestration with Ledger-Based Control for Improved LLM Coding Performance"* (Gao et al.).

Instead of relying on long-context interactive conversational loops that degrade context over time, `miniU` operates as an automated, deterministic orchestration harness. It uses a persistent ledger (`StateBoard`) and fresh context windows for every single reasoning and execution step.

⛔ **DON'T USE THIS IN PRODUCTION, IT'S JUST FOR LEARNING!!!.** ⛔

## ✨ Architecture & Features

- **🧠 Ledger-Based Control (`StateBoard`)**: A shared workspace state persisted via PostgreSQL detailing the high-level `Plan`, `TaskList`, `Notes`, and `WorkspaceArtifacts`.
- **🚀 Strict Context Isolation**: Zero conversational multi-turn history. Each invocation of the LLM receives only a freshly built state slice.
- **⚙️ Manager-Worker Loop**:
  - **Manager**: Evaluates the `problem.md` and current ledger, updates the plan, and curates atomic tasks (assigning statuses like `Pending` or `InProgress`).
  - **Worker**: Executes a single isolated task, reading code and notes to deterministically generate patches and write directly to disk.
- **🔍 Deterministic Verification Engine**: Automatically runs bash commands (e.g., `cargo test`) on the workspace and logs deterministic test outputs back into the ledger edge-cases to ground the LLM evaluation.
- **🔄 Multi-Model Support**: Allows independent configuration of powerful reasoning models for the Manager stage and fast execution models for the Worker stage.
- **🚀 Async & Fast**: Built on top of `tokio` and `reqwest`.

## 🛠️ Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/)
- **Runtime**: [Tokio](https://tokio.rs/)
- **HTTP Client**: [Reqwest](https://docs.rs/reqwest/)
- **Database**: [PostgreSQL](https://www.postgresql.org/) with [SQLx](https://github.com/launchbadge/sqlx)
- **Serialization**: [Serde](https://serde.rs/)

## 🚀 Getting Started

### Prerequisites

- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install)
- **PostgreSQL**: A running instance with a database created.
- **LLM Server**: A running OpenAI-compatible endpoint (e.g., vLLM or Ollama).

### Configuration

1. Create a `.env` file in the root directory and configure independent models for the distinct stages:

```env
DATABASE_URL=postgres://user:password@localhost/miniu_db
LLM_BASE_URL=127.0.0.1:8080
MANAGER_MODEL=your-reasoning-model-name
WORKER_MODEL=your-coding-model-name
```

2. The application will automatically initialize the necessary `sessions` and `session_state` tables on the first run.

### Running the App

To initiate an orchestration loop, provide a task instruction via a `problem.md` file in the root directory.

```bash
echo "Refactor math.rs to ensure the is_prime function handles negative integers correctly." > problem.md
cargo run
```

The orchestrator will run sequentially (Manager -> Worker -> Verifier) until all active tasks are completed or a round budget limit is met.

## 🏗️ Project Structure

- `src/main.rs`: Entry point and main automated Orchestrator loop.
- `src/modules/orchestrator.rs`: Implementation of the `manager_step` (state evaluation, planning, task curation).
- `src/modules/worker.rs`: Implementation of the `worker_step` (code generation and direct-to-disk patching).
- `src/modules/verifier.rs`: Deterministic verification engine wrapping `std::process::Command` (e.g., `cargo check / test`).
- `src/modules/memory/state_board.rs`: The central JSON-serializable Ledger managing Plans and Tasks.
- `src/traits/llm_client.rs`: OpenAI-compatible client implementation utilizing tool-calling for strict JSON outputs.

## 📜 License

This project is licensed under the MIT License - see the LICENSE file for details.