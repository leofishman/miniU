First draft to render StateBoards like this
{
  "version": 8,
  "last_update": "2026-05-11T13:20:00Z",
  "l1_immediate": {
    "last_user_intent": "define_planner_prompt",
    "temp_flags": ["waiting_for_code_standard_verification"],
    "retrieved_context": null
  },
  "l2_task": {
    "active_goal": "Implement StateBoard Core",
    "status": "executing",
    "progress": 0.65,
    "subtasks": [
      { "desc": "Define L1-L4 layers", "completed": true },
      { "desc": "Setup PostgreSQL schema", "completed": true },
      { "desc": "Implement Deep Merge logic in Rust", "completed": true },
      { "desc": "Define Planner System Prompt", "completed": true },
      { "desc": "Integrate Tool Calling for State Updates", "completed": false }
    ]
  },
  "l3_semantic": {
    "preferences": {
      "language": "Rust",
      "naming_convention": "English",
      "comments_language": "English"
    },
    "guardrails": [
      "No prefabricated empathy",
      "Direct to data and solutions",
      "Mono-LLM implementation first"
    ],
    "facts": [
      "Project name: miniU",
      "Backend: SQLx + Postgres",
      "LLM Engine: llama.cpp"
    ]
  },
  "l4_history": [
    { "id": "arch_001", "summary": "Refactoring discarded to focus on StateBoard implementation", "msg_ids": [1, 5] },
    { "id": "arch_002", "summary": "DB Schema updated with 'archived' and 'version' columns", "msg_ids": [10, 12] }
  ]
}