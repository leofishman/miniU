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

impl StateBoard {
    /// Funsiona un estado propuesto (del LLM o UI) con el estado actual de la DB.
    /// Si 'is_human' es true, sus cambios tienen prioridad absoluta.
    pub fn merge(&mut self, incoming: StateBoard, is_human: bool) {
        // 1. Meta & Version: Siempre incrementamos la versión global
        self.version += 1;
        self.last_update = Utc::now();

        // 2. L1 - Contexto Inmediato: El LLM es el autor principal, 
        // pero el humano puede resetearlo.
        if is_human {
            self.l1_immediate = incoming.l1_immediate;
        } else {
            // El LLM actualiza la intención y flags
            self.l1_immediate.last_user_intent = incoming.l1_immediate.last_user_intent;
            self.l1_immediate.temp_flags = incoming.l1_immediate.temp_flags;
            // El retrieved_context solo cambia si el LLM lo pide explícitamente
            if incoming.l1_immediate.retrieved_context.is_some() {
                self.l1_immediate.retrieved_context = incoming.l1_immediate.retrieved_context;
            }
        }

        // 3. L2 - Estado de Tarea: Fusión de sub-tareas
        if is_human {
            // Si el humano toca L2, su visión es la ley (ej. cancelar todo)
            self.l2_task = incoming.l2_task;
        } else {
            // El LLM actualiza progreso y estado
            self.l2_task.status = incoming.l2_task.status;
            self.l2_task.progress = incoming.l2_task.progress;
            
            // Merge inteligente de sub-tareas: no borramos las que el LLM no mencione
            // a menos que el LLM envíe una lista completa nueva.
            self.l2_task.subtasks = incoming.l2_task.subtasks;
        }

        // 4. L3 - Núcleo Semántico (Guardrails y Hechos)
        if is_human {
            // El humano es el "Owner" de las reglas
            self.l3_semantic = incoming.l3_semantic;
        } else {
            // El LLM solo puede AÑADIR hechos o preferencias, nunca borrar guardrails.
            for fact in incoming.l3_semantic.facts {
                if !self.l3_semantic.facts.contains(&fact) {
                    self.l3_semantic.facts.push(fact);
                }
            }
            // Fusionar preferencias (sin sobreescribir las del usuario)
            for (key, value) in incoming.l3_semantic.preferences {
                self.l3_semantic.preferences.entry(key).or_insert(value);
            }
        }

        // 5. L4 - Historia: El LLM solo hace Append (Añadir al final)
        for anchor in incoming.l4_history {
            if !self.l4_history.iter().any(|a| a.id == anchor.id) {
                self.l4_history.push(anchor);
            }
        }
    }
}