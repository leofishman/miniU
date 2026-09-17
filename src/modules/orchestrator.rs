use crate::modules::memory::StateBoard;
use crate::modules::memory::state_board::TaskStatus;
use crate::traits::llm_client::{ChatMessage, LlmClient, Role, get_update_state_tool};
use sqlx::PgPool;

pub async fn manager_step(
    problem_definition: &str,
    state_board: &mut StateBoard,
    client: &impl LlmClient,
    pool: &PgPool,
) -> Result<(), String> {
    let state_json = serde_json::to_string_pretty(state_board).unwrap_or_default();

    let system_prompt = format!(
        "You are the Manager for miniU.\n\
        Your role is to evaluate the current project state, create/revise the plan, curate the task list, and add notes if necessary.\n\
        You must ensure the task list correctly reflects what needs to be done next based on the Problem Definition.\n\
        Use the 'update_state' tool to modify the state board. DO NOT output conversational text.\n\n\
        ### Problem Definition:\n{}\n\n\
        ### Current State:\n{}",
        problem_definition, state_json
    );

    let messages = vec![
        ChatMessage {
            role: Role::System,
            content: system_prompt,
        },
        ChatMessage {
            role: Role::User,
            content: "Review the state and problem definition. If needed, update the state board using the tool. Set one task to 'InProgress' if execution is ready.".to_string(),
        },
    ];

    let tools = vec![get_update_state_tool()];

    if let Ok(Some(tool_call)) = client.chat_with_tools(&messages, &tools).await {
        if tool_call.function.name == "update_state" {
            match serde_json::from_str::<StateBoard>(&tool_call.function.arguments) {
                Ok(incoming_state) => {
                    state_board.merge(incoming_state);
                }
                Err(e) => {
                    println!("❌ Manager failed to parse state update JSON: {}", e);
                    println!("JSON was: {}", tool_call.function.arguments);
                }
            }
        }
    }

    Ok(())
}

pub fn is_done(state_board: &StateBoard) -> bool {
    // Basic termination condition: No pending or in-progress tasks
    // OR if we introduce a specific "Done" milestone
    !state_board.task_list.tasks.is_empty()
        && state_board
            .task_list
            .tasks
            .iter()
            .all(|t| t.status == TaskStatus::Completed)
}
