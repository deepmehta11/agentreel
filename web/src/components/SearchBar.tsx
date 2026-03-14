"use client";

import { useState, useEffect, useCallback } from "react";
import type { Step } from "@/lib/types";

export function SearchBar({
  steps,
  onResults,
}: {
  steps: Step[];
  onResults: (matchedIndexes: Set<number> | null) => void;
}) {
  const [query, setQuery] = useState("");
  const [matchCount, setMatchCount] = useState(0);

  const search = useCallback(
    (q: string) => {
      if (!q.trim()) {
        onResults(null);
        setMatchCount(0);
        return;
      }

      const lower = q.toLowerCase();
      const matched = new Set<number>();

      for (const step of steps) {
        const searchable = getSearchableText(step);
        if (searchable.toLowerCase().includes(lower)) {
          matched.add(step.index);
        }
      }

      setMatchCount(matched.size);
      onResults(matched.size > 0 ? matched : new Set());
    },
    [steps, onResults]
  );

  useEffect(() => {
    const timer = setTimeout(() => search(query), 150);
    return () => clearTimeout(timer);
  }, [query, search]);

  // Ctrl+F / Cmd+F handler
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "f") {
        e.preventDefault();
        document.getElementById("trajectory-search")?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <div className="flex items-center gap-3 px-4 py-2 bg-gray-900 border border-gray-800 rounded-lg">
      <svg className="w-4 h-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
      <input
        id="trajectory-search"
        type="text"
        placeholder="Search steps... (Cmd+F)"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="flex-1 bg-transparent text-sm text-gray-200 placeholder-gray-600 outline-none"
      />
      {query && (
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-500">
            {matchCount} {matchCount === 1 ? "match" : "matches"}
          </span>
          <button
            onClick={() => { setQuery(""); onResults(null); }}
            className="text-gray-500 hover:text-gray-300 text-xs"
          >
            Clear
          </button>
        </div>
      )}
    </div>
  );
}

function getSearchableText(step: Step): string {
  const parts: string[] = [step.type];

  if (step.llm_call) {
    parts.push(step.llm_call.model ?? "");
    parts.push(step.llm_call.provider ?? "");
    parts.push(step.llm_call.system_prompt ?? "");
    parts.push(step.llm_call.thinking ?? "");
    parts.push(step.llm_call.stop_reason ?? "");
    for (const msg of step.llm_call.messages ?? []) {
      parts.push(msg.role);
      parts.push(typeof msg.content === "string" ? msg.content : JSON.stringify(msg.content));
    }
    parts.push(step.llm_call.response?.content ?? "");
    for (const block of step.llm_call.response_blocks ?? []) {
      parts.push(block.type);
      parts.push(block.text ?? "");
      parts.push(block.tool_name ?? "");
      parts.push(JSON.stringify(block.input ?? ""));
    }
    for (const tool of step.llm_call.available_tools ?? []) {
      parts.push(tool.name);
      parts.push(tool.description ?? "");
    }
  }

  if (step.tool_call) {
    parts.push(step.tool_call.name);
    parts.push(step.tool_call.tool_type ?? "");
    parts.push(step.tool_call.mcp_server ?? "");
    parts.push(step.tool_call.mcp_server_name ?? "");
    parts.push(JSON.stringify(step.tool_call.input ?? ""));
    parts.push(typeof step.tool_call.output === "string" ? step.tool_call.output : JSON.stringify(step.tool_call.output));
  }

  if (step.tool_result) {
    parts.push(step.tool_result.tool_name);
    parts.push(typeof step.tool_result.output === "string" ? step.tool_result.output : JSON.stringify(step.tool_result.output));
  }

  if (step.thought) parts.push(step.thought.content);
  if (step.agent_decision) {
    parts.push(step.agent_decision.decision);
    parts.push(step.agent_decision.reasoning ?? "");
    parts.push((step.agent_decision.alternatives_considered ?? []).join(" "));
  }
  if (step.error) {
    parts.push(step.error.message ?? "");
    parts.push(step.error.code ?? "");
  }
  if (step.human_input) parts.push(step.human_input.content ?? "");
  if (step.file_operation) {
    parts.push(step.file_operation.path);
    parts.push(step.file_operation.content_preview ?? "");
  }

  return parts.join(" ");
}
