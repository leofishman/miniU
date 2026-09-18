mod modules;
mod traits;

use crate::modules::memory::OrchestrationSession;
use crate::modules::memory::StateBoard;
use crate::modules::memory::state_board::TaskStatus;
use crate::modules::orchestrator::{is_done, manager_step};
use crate::modules::verifier;
use crate::modules::worker::worker_step;
use crate::traits::llm_client::OpenAiClient;

use dotenvy::dotenv;
use sqlx::PgPool;
use std::env;
use std::fs;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL no definida");
    let llm_url = env::var("LLM_BASE_URL").expect("LLM_BASE_URL no definida");

    let manager_model =
        env::var("MANAGER_MODEL").unwrap_or_else(|_| "manager-model-default".to_string());
    let worker_model =
        env::var("WORKER_MODEL").unwrap_or_else(|_| "worker-model-default".to_string());

    let pool = PgPool::connect(&database_url).await?;
    crate::modules::memory::database::init_db(&pool).await?;

    let manager_client = OpenAiClient {
        api_key: "".to_string(),
        base_url: llm_url.clone(),
        model: manager_model,
        client: reqwest::Client::new(),
    };

    let worker_client = OpenAiClient {
        api_key: "".to_string(),
        base_url: llm_url,
        model: worker_model,
        client: reqwest::Client::new(),
    };

    let session_id = Uuid::new_v4();

    // Start fresh state or load existing
    let mut session =
        OrchestrationSession::new(manager_client, worker_client, session_id, &pool).await?;

    if session.state_board.is_none() {
        let initial_state = StateBoard::default();
        crate::modules::memory::database::init_session_state(&pool, &session_id, &initial_state)
            .await?;
        session.state_board = Some(initial_state);
    }

    let problem_definition = fs::read_to_string("problem.md")
        .unwrap_or_else(|_| "No problem.md found. Proceeding with default state.".to_string());

    let max_rounds = 10;
    let mut rounds = 0;

    println!("🚀 Starting Zero-Shot Orchestration loop...\n");

    loop {
        if rounds >= max_rounds {
            println!("❌ Max round budget reached. Terminating.");
            break;
        }

        rounds += 1;
        println!("==== ROUND {} ====", rounds);

        let mut state_board = session.state_board.take().unwrap();

        // 1. Manager Step
        println!("🧠 Manager evaluating state...");
        manager_step(
            &problem_definition,
            &mut state_board,
            &session.manager_client,
            &pool,
        )
        .await?;

        // 2. Check Completion Criteria
        if is_done(&state_board) {
            println!("✅ Manager decides we are done. Terminating.");
            session.state_board = Some(state_board);
            break;
        }

        // 3. Worker Step (find active task)
        let active_task_index = state_board
            .task_list
            .tasks
            .iter()
            .position(|t| t.status == TaskStatus::InProgress);

        if let Some(index) = active_task_index {
            // Clone task so we can borrow state_board mutably in worker_step
            let task = state_board.task_list.tasks[index].clone();
            println!("⚙️ Worker executing task: {}", task.description);
            if let Err(e) =
                worker_step(&task, &mut state_board, &session.worker_client, &pool).await
            {
                println!("❌ Worker failed: {}", e);
                state_board.task_list.tasks[index].status = TaskStatus::Failed;
            } else {
                println!("✅ Worker finished writing files.");
                state_board.task_list.tasks[index].status = TaskStatus::Completed;
            }
        } else {
            println!("⚠️ No task InProgress. Manager must curate tasks better.");
        }

        // 4. Verifier Step
        // Determine what command to run. Defaulting to `cargo check` for rust projects for now.
        // We could extract this from the Problem Definition or State Board dynamically.
        println!("🔍 Verifying codebase...");

        // We assume we run test on one of the workspace codes or simply a global project test
        let test_cmd = "cargo test";

        match verifier::run_tests(test_cmd) {
            Ok(result) => {
                println!(
                    "🧪 Test Result: {}",
                    if result.success { "PASS" } else { "FAIL" }
                );
                state_board.notes.edge_cases.push(format!(
                    "Test Result ({}): Success={}",
                    test_cmd, result.success
                ));
                if !result.success {
                    state_board
                        .notes
                        .edge_cases
                        .push(format!("Stderr: {}", result.stderr));
                }
            }
            Err(e) => {
                println!("❌ Verifier encountered an error: {}", e);
                state_board
                    .notes
                    .edge_cases
                    .push(format!("Test Execution Error: {}", e));
            }
        }

        // Save StateBoard
        crate::modules::memory::database::update_state_board(
            &pool,
            &session_id,
            state_board.clone(),
        )
        .await?;

        session.state_board = Some(state_board);
        println!("\n");
    }

    Ok(())
}
