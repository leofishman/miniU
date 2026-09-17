use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StateBoard {
    pub version: u64,
    pub last_update: DateTime<Utc>,
    pub plan: Plan,
    pub task_list: TaskList,
    pub notes: Notes,
    pub workspace: WorkspaceArtifacts,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Plan {
    pub execution_strategy: String,
    pub milestones: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub assigned_worker: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TaskList {
    pub tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Notes {
    pub architectural_decisions: Vec<String>,
    pub interfaces: Vec<String>,
    pub edge_cases: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WorkspaceArtifacts {
    pub code_paths: Vec<String>,
    pub diff_paths: Vec<String>,
    pub test_paths: Vec<String>,
}

impl StateBoard {
    pub fn merge(&mut self, incoming: StateBoard) {
        self.version += 1;
        self.last_update = Utc::now();

        // Deep-merging logic
        self.plan = incoming.plan;

        // Merge Tasks
        for incoming_task in incoming.task_list.tasks {
            if let Some(existing_task) = self
                .task_list
                .tasks
                .iter_mut()
                .find(|t| t.id == incoming_task.id)
            {
                existing_task.description = incoming_task.description;
                existing_task.status = incoming_task.status;
                existing_task.assigned_worker = incoming_task.assigned_worker;
            } else {
                self.task_list.tasks.push(incoming_task);
            }
        }

        // Merge Notes
        for decision in incoming.notes.architectural_decisions {
            if !self.notes.architectural_decisions.contains(&decision) {
                self.notes.architectural_decisions.push(decision);
            }
        }
        for interface in incoming.notes.interfaces {
            if !self.notes.interfaces.contains(&interface) {
                self.notes.interfaces.push(interface);
            }
        }
        for edge_case in incoming.notes.edge_cases {
            if !self.notes.edge_cases.contains(&edge_case) {
                self.notes.edge_cases.push(edge_case);
            }
        }

        // Update workspace
        self.workspace = incoming.workspace;
    }
}
