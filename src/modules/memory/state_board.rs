use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StateBoard {
    pub version: u64,
    pub last_update: DateTime<Utc>,
    pub l1_immediate: L1Context,
    pub l2_task: L2State,
    pub l3_semantic: L3Core,
    pub l4_history: Vec<HistoryAnchor>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L1Context {
    pub last_user_intent: String,
    pub temp_flags: Vec<String>,
    pub retrieved_context: Option<String>, 
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L2State {
    pub active_goal: String,
    pub status: String, // "thinking", "executing", "blocked", "done"
    pub progress: f32,   // 0.0 to 1.0
    pub subtasks: Vec<Subtask>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Subtask {
    pub desc: String,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L3Core {
    pub preferences: HashMap<String, String>,
    pub guardrails: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryAnchor {
    pub id: String,
    pub summary: String,
    pub msg_ids: Vec<i32>,
}