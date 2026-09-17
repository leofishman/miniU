use crate::modules::memory::StateBoard;
use crate::modules::memory::state_board::Task;
use crate::traits::llm_client::{ChatMessage, LlmClient, Role};
use sqlx::PgPool;

use std::fs;

pub async fn worker_step(
    task: &Task,
    state_board: &mut StateBoard,
    client: &impl LlmClient,
    pool: &PgPool,
) -> Result<(), String> {
    // Collect the files from the workspace artifacts that are relevant to this task
    let mut files_content = String::new();
    for path in &state_board.workspace.code_paths {
        if let Ok(content) = fs::read_to_string(path) {
            files_content.push_str(&format!("\n--- {} ---\n{}\n", path, content));
        }
    }

    let notes = &state_board.notes;
    let notes_json = serde_json::to_string_pretty(notes).unwrap_or_default();

    let system_prompt = format!(
        "You are the execution Worker for miniU.\n\
        Your sole responsibility is to complete the assigned task by generating the full updated source code.\n\
        You must output ONLY the updated code, with NO markdown formatting, no explanation, no conversational text.\n\
        The file content should be output as it is, ready to be saved to the disk.\n\n\
        ### Task:\n{}\n\n\
        ### Relevant Notes:\n{}\n\n\
        ### Current Files:\n{}",
        task.description, notes_json, files_content
    );

    let messages = vec![
        ChatMessage {
            role: Role::System,
            content: system_prompt,
        },
        ChatMessage {
            role: Role::User,
            content: "Provide the modified file content based on the task.".to_string(),
        },
    ];

    let response = client.chat(&messages, pool).await?;

    // Parse the markdown block if it exists
    let extracted_code = if response.contains("```") {
        let lines: Vec<&str> = response.lines().collect();
        let mut code_lines = Vec::new();
        let mut in_code_block = false;

        for line in lines {
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if in_code_block {
                code_lines.push(line);
            }
        }
        code_lines.join("\n")
    } else {
        response.clone()
    };

    // Since the worker outputs the full updated code, we need to know WHICH file to update.
    // To simplify for now, let's assume there is exactly one file in code_paths or the first file is the target.
    // In a more robust system, the LLM could output a JSON structure or diff.
    // Here we'll overwrite the first relevant code_path found.
    if let Some(target_path) = state_board.workspace.code_paths.first() {
        fs::write(target_path, extracted_code.trim())
            .map_err(|e| format!("Failed to write to file {}: {}", target_path, e))?;
    } else {
        return Err("No code paths available in workspace to write to.".into());
    }

    Ok(())
}
