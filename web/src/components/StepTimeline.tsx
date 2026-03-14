"use client";

import type { Step, LlmCallData } from "@/lib/types";
import { formatDuration, stepIcon } from "@/lib/parse";
import { useState, useRef, useEffect } from "react";
import { Markdown } from "./Markdown";
import { JsonTree } from "./JsonTree";

export function StepTimeline({
  steps,
  matchedIndexes,
  expandedIndex,
}: {
  steps: Step[];
  matchedIndexes?: Set<number> | null;
  expandedIndex?: number | null;
}) {
  return (
    <div className="space-y-2">
      {steps.map((step) => {
        const hidden = matchedIndexes != null && !matchedIndexes.has(step.index);
        return (
          <StepRow
            key={step.index}
            step={step}
            hidden={hidden}
            forceExpanded={expandedIndex === step.index}
          />
        );
      })}
    </div>
  );
}

function StepRow({
  step,
  hidden,
  forceExpanded,
}: {
  step: Step;
  hidden?: boolean;
  forceExpanded?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const [showRaw, setShowRaw] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (forceExpanded) {
      setExpanded(true);
      ref.current?.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [forceExpanded]);

  if (hidden) return null;

  const icon = stepIcon(step.type);
  const duration = step.duration_ms ? formatDuration(step.duration_ms) : "";

  const tokens = step.tokens
    ? `${step.tokens.input_tokens ?? 0} in / ${step.tokens.output_tokens ?? 0} out`
    : "";

  let label = "";
  switch (step.type) {
    case "llm_call":
      label = `LLM call (${step.llm_call?.model ?? "unknown"})`;
      break;
    case "tool_call":
      label = step.tool_call?.name ?? "tool";
      if (step.tool_call?.status) label += ` (${step.tool_call.status})`;
      if (step.tool_call?.tool_type === "mcp") label += ` [MCP]`;
      break;
    case "tool_result":
      label = `Result: ${step.tool_result?.tool_name ?? "?"}`;
      if (step.tool_result?.is_error) label += " (error)";
      break;
    case "thought":
      label = step.thought?.content?.slice(0, 80) ?? "thinking...";
      break;
    case "agent_decision":
      label = `Decision: ${step.agent_decision?.decision?.slice(0, 80) ?? "..."}`;
      break;
    case "file_operation":
      label = `${step.file_operation?.operation ?? "file"} ${step.file_operation?.path ?? ""}`;
      break;
    case "human_input":
      label = `Human ${step.human_input?.action ?? "input"}`;
      break;
    case "error":
      label = `Error: ${step.error?.message ?? "unknown"}`;
      break;
    default:
      label = step.type;
  }

  const hasDetail =
    step.type === "llm_call" ||
    step.type === "tool_call" ||
    step.type === "tool_result" ||
    step.type === "thought" ||
    step.type === "agent_decision" ||
    step.type === "error" ||
    step.type === "file_operation";

  return (
    <div ref={ref} className="border border-gray-800 rounded-lg overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-3 px-4 py-3 hover:bg-gray-900/50 text-left"
      >
        <span className="text-lg">{icon}</span>
        <span className="font-mono text-xs text-gray-500 w-8">#{step.index}</span>
        <span className="flex-1 text-sm truncate">{label}</span>
        {step.cost_usd != null && step.cost_usd > 0 && (
          <span className="text-xs text-green-400">${step.cost_usd.toFixed(4)}</span>
        )}
        {tokens && <span className="text-xs text-gray-500">{tokens}</span>}
        {duration && (
          <span className="text-xs text-gray-500 w-16 text-right">{duration}</span>
        )}
        <span className="text-gray-600 text-xs">{expanded ? "\u25B2" : "\u25BC"}</span>
      </button>

      {expanded && hasDetail && (
        <div className="border-t border-gray-800 bg-gray-900/50">
          {step.type === "llm_call" && step.llm_call && (
            <LlmCallDetail call={step.llm_call} showRaw={showRaw} />
          )}
          {step.type === "tool_call" && step.tool_call && (
            <ToolCallDetail call={step.tool_call} />
          )}
          {step.type === "tool_result" && step.tool_result && (
            <div className="px-4 py-3">
              <Label>Output</Label>
              <Pre>{typeof step.tool_result.output === "string" ? step.tool_result.output : JSON.stringify(step.tool_result.output, null, 2)}</Pre>
            </div>
          )}
          {step.type === "thought" && step.thought && (
            <div className="px-4 py-3">
              <Pre>{step.thought.content}</Pre>
            </div>
          )}
          {step.type === "agent_decision" && step.agent_decision && (
            <div className="px-4 py-3 space-y-2">
              <div><Label>Decision</Label><span className="text-sm">{step.agent_decision.decision}</span></div>
              {step.agent_decision.reasoning && (
                <div><Label>Reasoning</Label><span className="text-sm text-gray-400">{step.agent_decision.reasoning}</span></div>
              )}
              {step.agent_decision.alternatives_considered && step.agent_decision.alternatives_considered.length > 0 && (
                <div>
                  <Label>Alternatives</Label>
                  <ul className="text-sm text-gray-500 list-disc ml-4">
                    {step.agent_decision.alternatives_considered.map((a, i) => (
                      <li key={i}>{a}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
          {step.type === "error" && step.error && (
            <div className="px-4 py-3 space-y-1">
              {step.error.code && <div><Label>Code</Label><span className="text-sm text-red-400">{step.error.code}</span></div>}
              <div><Label>Message</Label><span className="text-sm">{step.error.message}</span></div>
              {step.error.stack_trace && <Pre>{step.error.stack_trace}</Pre>}
            </div>
          )}
          {step.type === "file_operation" && step.file_operation && (
            <div className="px-4 py-3 space-y-1">
              <div><Label>Path</Label><span className="text-sm font-mono">{step.file_operation.path}</span></div>
              {step.file_operation.size_bytes != null && <div><Label>Size</Label><span className="text-sm">{step.file_operation.size_bytes} bytes</span></div>}
              {step.file_operation.content_preview && <Pre>{step.file_operation.content_preview}</Pre>}
            </div>
          )}

          {/* Raw toggle for LLM calls */}
          {step.type === "llm_call" && step.llm_call?.http && (
            <div className="px-4 py-2 border-t border-gray-800">
              <button
                onClick={() => setShowRaw(!showRaw)}
                className="text-xs text-brand-400 hover:text-brand-300"
              >
                {showRaw ? "Hide raw HTTP" : "Show raw HTTP"}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function LlmCallDetail({ call, showRaw }: { call: LlmCallData; showRaw: boolean }) {
  return (
    <div className="divide-y divide-gray-800/50">
      {/* Config bar */}
      {(call.config || call.provider || call.system_prompt || call.available_tools?.length) && (
        <div className="px-4 py-3 space-y-2">
          {/* Provider + Model */}
          <div className="flex flex-wrap gap-4 text-xs">
            {call.provider && (
              <span><Label>Provider</Label>{call.provider}</span>
            )}
            {call.model && (
              <span><Label>Model</Label>{call.model}</span>
            )}
            {call.stop_reason && (
              <span><Label>Stop</Label><span className="text-yellow-400">{call.stop_reason}</span></span>
            )}
          </div>

          {/* Config params */}
          {call.config && (
            <div className="flex flex-wrap gap-3 text-xs text-gray-500">
              {call.config.temperature != null && <span>temp={call.config.temperature}</span>}
              {call.config.max_tokens != null && <span>max_tokens={call.config.max_tokens}</span>}
              {call.config.top_p != null && <span>top_p={call.config.top_p}</span>}
            </div>
          )}

          {/* Available tools */}
          {call.available_tools && call.available_tools.length > 0 && (
            <div className="text-xs">
              <Label>Tools</Label>
              <span className="text-gray-400">
                {call.available_tools.map((t) => t.name).join(", ")}
              </span>
            </div>
          )}
        </div>
      )}

      {/* System prompt */}
      {call.system_prompt && (
        <div className="px-4 py-3">
          <Label>System Prompt</Label>
          <Pre>{call.system_prompt}</Pre>
        </div>
      )}

      {/* Input messages */}
      {call.messages && call.messages.length > 0 && (
        <div className="px-4 py-3 space-y-2">
          <Label>Messages</Label>
          {call.messages.map((msg, i) => (
            <div key={i} className="flex gap-2 text-xs">
              <span className={`font-mono w-16 shrink-0 ${
                msg.role === "user" ? "text-blue-400" :
                msg.role === "assistant" ? "text-green-400" :
                msg.role === "system" ? "text-yellow-400" :
                "text-gray-400"
              }`}>
                {msg.role}
              </span>
              <span className="text-gray-300 whitespace-pre-wrap break-words">
                {typeof msg.content === "string" ? msg.content : JSON.stringify(msg.content, null, 2)}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Thinking */}
      {call.thinking && (
        <div className="px-4 py-3">
          <Label>{"\uD83D\uDCAD"} Internal Reasoning</Label>
          <div className="mt-1 p-3 bg-purple-900/20 border border-purple-800/30 rounded text-xs text-purple-200 whitespace-pre-wrap">
            {call.thinking}
          </div>
        </div>
      )}

      {/* Response blocks */}
      {call.response_blocks && call.response_blocks.length > 0 ? (
        <div className="px-4 py-3 space-y-2">
          <Label>Response</Label>
          {call.response_blocks.map((block, i) => (
            <div key={i}>
              {block.type === "text" && block.text && (
                <div className="p-3 bg-gray-800/50 rounded text-xs text-gray-200">
                  <Markdown content={block.text} />
                </div>
              )}
              {block.type === "thinking" && block.text && (
                <div className="p-3 bg-purple-900/20 border border-purple-800/30 rounded text-xs text-purple-200 whitespace-pre-wrap">
                  <span className="text-purple-400 font-medium">{"\uD83D\uDCAD"} thinking: </span>
                  {block.text}
                </div>
              )}
              {block.type === "tool_use" && (
                <div className="p-3 bg-amber-900/20 border border-amber-800/30 rounded text-xs">
                  <span className="text-amber-400 font-medium">{"\uD83D\uDD27"} {block.tool_name}</span>
                  {block.tool_use_id && (
                    <span className="text-gray-600 ml-2">[{block.tool_use_id}]</span>
                  )}
                  {block.input && (
                    <pre className="mt-1 text-gray-300">{JSON.stringify(block.input, null, 2)}</pre>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      ) : call.response?.content ? (
        <div className="px-4 py-3">
          <Label>Response</Label>
          <div className="p-3 bg-gray-800/50 rounded text-xs text-gray-200">
            <Markdown content={call.response.content} />
          </div>
        </div>
      ) : null}

      {/* HTTP details */}
      {showRaw && call.http && <HttpDetail http={call.http} />}
    </div>
  );
}

function ToolCallDetail({ call }: { call: Step["tool_call"] }) {
  if (!call) return null;
  return (
    <div className="px-4 py-3 space-y-2">
      <div className="flex gap-4 text-xs">
        {call.tool_type && <span><Label>Type</Label>{call.tool_type}</span>}
        {call.mcp_server_name && <span><Label>MCP Server</Label>{call.mcp_server_name}</span>}
        {call.mcp_server && !call.mcp_server_name && <span><Label>MCP</Label>{call.mcp_server}</span>}
      </div>
      {call.input && Object.keys(call.input).length > 0 && (
        <div>
          <Label>Input</Label>
          <div className="mt-1 p-2 bg-gray-950 rounded border border-gray-800">
            <JsonTree data={call.input} defaultExpanded={2} />
          </div>
        </div>
      )}
      {call.output != null && (
        <div>
          <Label>Output</Label>
          {typeof call.output === "string" ? (
            <div className="mt-1 p-2 bg-gray-950 rounded border border-gray-800 text-xs text-gray-300">
              <Markdown content={call.output} />
            </div>
          ) : (
            <div className="mt-1 p-2 bg-gray-950 rounded border border-gray-800">
              <JsonTree data={call.output} defaultExpanded={2} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function Label({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-[10px] uppercase tracking-wider text-gray-600 mr-2 font-medium">
      {children}
    </span>
  );
}

function HttpDetail({ http }: { http: NonNullable<LlmCallData["http"]> }) {
  const respHeaders = http.response_headers ?? {};
  return (
    <div className="px-4 py-3 space-y-3">
      <div className="flex items-center gap-2 text-xs">
        <span className="font-mono text-blue-400">
          {http.method} {http.url}
        </span>
        <span className={`px-1.5 py-0.5 rounded text-xs ${
          http.status_code === 200 ? "bg-green-900/50 text-green-400" : "bg-red-900/50 text-red-400"
        }`}>
          {http.status_code}
        </span>
      </div>

      {Object.keys(respHeaders).length > 0 && (
        <div>
          <Label>Response Headers</Label>
          <div className="mt-1 grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs font-mono">
            {Object.entries(respHeaders).map(([k, v]) => (
              <div key={k} className="contents">
                <span className="text-gray-500 truncate">{k}</span>
                <span className="text-gray-300 truncate">{String(v)}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {http.request_body != null && (
        <div>
          <Label>Request Body</Label>
          <div className="mt-1 p-2 bg-gray-950 rounded border border-gray-800 max-h-96 overflow-auto">
            <JsonTree data={http.request_body} defaultExpanded={1} />
          </div>
        </div>
      )}

      {http.response_body != null && (
        <div>
          <Label>Response Body</Label>
          <div className="mt-1 p-2 bg-gray-950 rounded border border-gray-800 max-h-96 overflow-auto">
            <JsonTree data={http.response_body} defaultExpanded={1} />
          </div>
        </div>
      )}
    </div>
  );
}

function ResponseHeaders({ headers }: { headers?: Record<string, string> }) {
  if (!headers || Object.keys(headers).length === 0) return null;
  return (
    <div>
      <Label>Response Headers</Label>
      <div className="mt-1 grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs font-mono">
        {Object.entries(headers).map(([k, v]) => (
          <div key={k} className="contents">
            <span className="text-gray-500 truncate">{k}</span>
            <span className="text-gray-300 truncate">{v}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function Pre({ children }: { children: React.ReactNode }) {
  return (
    <pre className="mt-1 text-xs text-gray-300 whitespace-pre-wrap break-words max-h-96 overflow-auto p-2 bg-gray-950 rounded border border-gray-800">
      {children}
    </pre>
  );
}
