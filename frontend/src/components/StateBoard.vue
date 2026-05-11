<script setup>
import { computed } from 'vue';

const state = {
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
};

const progressWidth = computed(() => `${state.l2_task.progress * 100}%`);
</script>

<template>
  <div class="min-h-screen bg-slate-900 text-slate-200 p-6 font-mono text-sm">
    <!-- Header -->
    <header class="flex justify-between items-center border-b border-slate-700 pb-4 mb-6">
      <h1 class="text-xl font-bold text-blue-400">miniU :: StateBoard v{{ state.version }}</h1>
      <span class="text-slate-500">Last Update: {{ new Date(state.last_update).toLocaleString() }}</span>
    </header>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      
      <!-- L1: Immediate Context -->
      <section class="bg-slate-800 p-4 rounded border-t-2 border-red-500">
        <h2 class="text-red-400 font-bold mb-3 underline">L1: Immediate</h2>
        <div class="space-y-2">
          <p><span class="text-slate-500">Intent:</span> {{ state.l1_immediate.last_user_intent }}</p>
          <div>
            <p class="text-slate-500 mb-1">Flags:</p>
            <ul class="list-disc pl-4 italic">
              <li v-for="flag in state.l1_immediate.temp_flags" :key="flag">{{ flag }}</li>
            </ul>
          </div>
        </div>
      </section>

      <!-- L2: Task/Execution -->
      <section class="bg-slate-800 p-4 rounded border-t-2 border-green-500 lg:col-span-1">
        <h2 class="text-green-400 font-bold mb-3 underline">L2: Task</h2>
        <p class="font-bold text-white mb-2">{{ state.l2_task.active_goal }}</p>
        
        <!-- Progress Bar -->
        <div class="w-full bg-slate-700 h-2 rounded mb-4">
          <div class="bg-green-500 h-2 rounded transition-all duration-500" :style="{ width: progressWidth }"></div>
        </div>

        <ul class="space-y-1">
          <li v-for="task in state.l2_task.subtasks" :key="task.desc" class="flex items-start gap-2">
            <span :class="task.completed ? 'text-green-500' : 'text-slate-600'">
              {{ task.completed ? '☑' : '☐' }}
            </span>
            <span :class="{ 'text-slate-500 line-through': task.completed }">{{ task.desc }}</span>
          </li>
        </ul>
      </section>

      <!-- L3: Semantic/Guardrails -->
      <section class="bg-slate-800 p-4 rounded border-t-2 border-blue-500 lg:col-span-1">
        <h2 class="text-blue-400 font-bold mb-3 underline">L3: Semantic</h2>
        <div class="mb-4">
          <h3 class="text-xs uppercase text-slate-500 font-bold">Guardrails</h3>
          <ul class="list-none text-yellow-500/80">
            <li v-for="g in state.l3_semantic.guardrails" :key="g">⚠ {{ g }}</li>
          </ul>
        </div>
        <div>
          <h3 class="text-xs uppercase text-slate-500 font-bold">System Facts</h3>
          <ul class="list-none italic">
            <li v-for="fact in state.l3_semantic.facts" :key="fact">i: {{ fact }}</li>
          </ul>
        </div>
      </section>

      <!-- L4: History/Archival -->
      <section class="bg-slate-800 p-4 rounded border-t-2 border-purple-500 overflow-y-auto max-h-96">
        <h2 class="text-purple-400 font-bold mb-3 underline">L4: History</h2>
        <div v-for="entry in state.l4_history" :key="entry.id" class="mb-3 p-2 bg-slate-900/50 rounded border border-slate-700">
          <span class="text-[10px] text-purple-300 block mb-1">#{{ entry.id }}</span>
          <p class="text-xs">{{ entry.summary }}</p>
        </div>
      </section>

    </div>
  </div>
</template>
