"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import type { Step } from "@/lib/types";
import { formatDuration, stepIcon } from "@/lib/parse";
import { Markdown } from "./Markdown";

export function ReplayDebugger({
  steps,
  onClose,
}: {
  steps: Step[];
  onClose: () => void;
}) {
  const [currentStep, setCurrentStep] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(1);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  const step = steps[currentStep];
  const progress = steps.length > 1 ? (currentStep / (steps.length - 1)) * 100 : 100;

  // Cumulative stats up to current step
  const cumulative = steps.slice(0, currentStep + 1).reduce(
    (acc, s) => ({
      tokens: acc.tokens + (s.tokens?.input_tokens ?? 0) + (s.tokens?.output_tokens ?? 0),
      cost: acc.cost + (s.cost_usd ?? 0),
      duration: acc.duration + (s.duration_ms ?? 0),
      llmCalls: acc.llmCalls + (s.type === "llm_call" ? 1 : 0),
      toolCalls: acc.toolCalls + (s.type === "tool_call" ? 1 : 0),
      errors: acc.errors + (s.type === "error" ? 1 : 0),
    }),
    { tokens: 0, cost: 0, duration: 0, llmCalls: 0, toolCalls: 0, errors: 0 }
  );

  const goNext = useCallback(() => {
    setCurrentStep((prev) => Math.min(prev + 1, steps.length - 1));
  }, [steps.length]);

  const goPrev = useCallback(() => {
    setCurrentStep((prev) => Math.max(prev - 1, 0));
  }, []);

  // Auto-play
  useEffect(() => {
    if (playing && currentStep < steps.length - 1) {
      const delay = (steps[currentStep]?.duration_ms ?? 1000) / speed;
      const clampedDelay = Math.min(Math.max(delay, 200), 3000);
      timerRef.current = setTimeout(() => {
        goNext();
      }, clampedDelay);
    } else if (currentStep >= steps.length - 1) {
      setPlaying(false);
    }
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [playing, currentStep, speed, steps, goNext]);

  // Keyboard controls
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight" || e.key === "l") goNext();
      else if (e.key === "ArrowLeft" || e.key === "h") goPrev();
      else if (e.key === " ") { e.preventDefault(); setPlaying((p) => !p); }
      else if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [goNext, goPrev, onClose]);

  if (!step) return null;

  return (
    <div className="fixed inset-0 z-50 bg-gray-950/95 flex flex-col">
      {/* Top bar */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-gray-800 bg-gray-900">
        <div className="flex items-center gap-4">
          <span className="text-brand-400 font-bold">Replay</span>
          <span className="text-sm text-gray-400">
            Step {currentStep + 1} of {steps.length}
          </span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-600">Speed:</span>
          {[0.5, 1, 2, 5].map((s) => (
            <button
              key={s}
              onClick={() => setSpeed(s)}
              className={`text-xs px-2 py-0.5 rounded ${
                speed === s ? "bg-brand-600 text-white" : "text-gray-500 hover:text-gray-300"
              }`}
            >
              {s}x
            </button>
          ))}
          <button onClick={onClose} className="ml-4 text-gray-500 hover:text-white text-sm">
            Close (Esc)
          </button>
        </div>
      </div>

      {/* Progress bar */}
      <div className="h-1 bg-gray-800">
        <div
          className="h-full bg-brand-500 transition-all duration-300"
          style={{ width: `${progress}%` }}
        />
      </div>

      {/* Main content */}
      <div className="flex-1 overflow-auto px-6 py-6">
        <div className="max-w-4xl mx-auto space-y-6">
          {/* Cumulative stats */}
          <div className="grid grid-cols-6 gap-4 text-center">
            <MiniStat label="Step" value={`${currentStep + 1}/${steps.length}`} />
            <MiniStat label="LLM Calls" value={String(cumulative.llmCalls)} />
            <MiniStat label="Tool Calls" value={String(cumulative.toolCalls)} />
            <MiniStat label="Tokens" value={cumulative.tokens > 1000 ? `${(cumulative.tokens / 1000).toFixed(1)}k` : String(cumulative.tokens)} />
            <MiniStat label="Cost" value={`$${cumulative.cost.toFixed(4)}`} color="text-green-400" />
            <MiniStat label="Duration" value={formatDuration(cumulative.duration)} />
          </div>

          {/* Current step */}
          <div className="border border-gray-800 rounded-lg overflow-hidden">
            <div className="px-4 py-3 bg-gray-900 flex items-center gap-3">
              <span className="text-2xl">{stepIcon(step.type)}</span>
              <div>
                <div className="text-sm font-medium">{stepTitle(step)}</div>
                <div className="text-xs text-gray-500">
                  {step.timestamp} {step.duration_ms ? `· ${formatDuration(step.duration_ms)}` : ""}
                </div>
              </div>
            </div>
            <div className="px-4 py-4">
              <StepContent step={step} />
            </div>
          </div>

          {/* Mini timeline at bottom */}
          <div className="flex gap-0.5">
            {steps.map((s, i) => (
              <button
                key={i}
                onClick={() => setCurrentStep(i)}
                className={`flex-1 h-2 rounded-sm transition-colors ${
                  i === currentStep
                    ? "bg-brand-500"
                    : i < currentStep
                      ? "bg-gray-700"
                      : "bg-gray-800"
                }`}
                title={`Step ${i}: ${s.type}`}
              />
            ))}
          </div>
        </div>
      </div>

      {/* Controls */}
      <div className="flex items-center justify-center gap-4 px-6 py-4 border-t border-gray-800 bg-gray-900">
        <button
          onClick={() => setCurrentStep(0)}
          className="text-gray-500 hover:text-white text-sm px-3 py-1"
        >
          |&lt;
        </button>
        <button
          onClick={goPrev}
          disabled={currentStep === 0}
          className="text-gray-400 hover:text-white disabled:text-gray-700 text-lg px-3 py-1"
        >
          &larr;
        </button>
        <button
          onClick={() => setPlaying(!playing)}
          className="bg-brand-600 hover:bg-brand-500 text-white px-6 py-2 rounded-lg text-sm font-medium min-w-[80px]"
        >
          {playing ? "Pause" : "Play"}
        </button>
        <button
          onClick={goNext}
          disabled={currentStep >= steps.length - 1}
          className="text-gray-400 hover:text-white disabled:text-gray-700 text-lg px-3 py-1"
        >
          &rarr;
        </button>
        <button
          onClick={() => setCurrentStep(steps.length - 1)}
          className="text-gray-500 hover:text-white text-sm px-3 py-1"
        >
          &gt;|
        </button>
        <span className="text-xs text-gray-600 ml-4">
          Space: play/pause · Arrow keys: step · Esc: close
        </span>
      </div>
    </div>
  );
}

function MiniStat({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div className="bg-gray-900 rounded-lg px-3 py-2">
      <div className="text-[10px] text-gray-600 uppercase tracking-wider">{label}</div>
      <div className={`text-sm font-medium ${color ?? "text-gray-200"}`}>{value}</div>
    </div>
  );
}

function stepTitle(step: Step): string {
  switch (step.type) {
    case "llm_call":
      return `LLM Call — ${step.llm_call?.model ?? "unknown"}`;
    case "tool_call":
      return `Tool Call — ${step.tool_call?.name ?? "unknown"}`;
    case "tool_result":
      return `Tool Result — ${step.tool_result?.tool_name ?? "unknown"}`;
    case "thought":
      return "Agent Thinking";
    case "agent_decision":
      return "Agent Decision";
    case "error":
      return `Error — ${step.error?.message ?? "unknown"}`;
    case "human_input":
      return `Human Input — ${step.human_input?.action ?? "message"}`;
    case "file_operation":
      return `File — ${step.file_operation?.operation} ${step.file_operation?.path}`;
    default:
      return step.type;
  }
}

function StepContent({ step }: { step: Step }) {
  switch (step.type) {
    case "llm_call": {
      const call = step.llm_call;
      if (!call) return null;
      return (
        <div className="space-y-4 text-sm">
          {call.system_prompt && (
            <div>
              <div className="text-[10px] uppercase text-gray-600 mb-1">System Prompt</div>
              <div className="p-3 bg-yellow-900/10 border border-yellow-800/20 rounded text-xs text-yellow-200">
                {call.system_prompt}
              </div>
            </div>
          )}
          {call.messages?.map((msg, i) => (
            <div key={i}>
              <div className={`text-[10px] uppercase mb-1 ${
                msg.role === "user" ? "text-blue-500" : msg.role === "assistant" ? "text-green-500" : "text-gray-500"
              }`}>
                {msg.role}
              </div>
              <div className="p-3 bg-gray-800/50 rounded text-xs">
                <Markdown content={typeof msg.content === "string" ? msg.content : JSON.stringify(msg.content)} />
              </div>
            </div>
          ))}
          {call.thinking && (
            <div>
              <div className="text-[10px] uppercase text-purple-500 mb-1">Internal Reasoning</div>
              <div className="p-3 bg-purple-900/20 border border-purple-800/30 rounded text-xs text-purple-200 whitespace-pre-wrap">
                {call.thinking}
              </div>
            </div>
          )}
          {(call.response?.content || (call.response_blocks && call.response_blocks.length > 0)) && (
            <div>
              <div className="text-[10px] uppercase text-green-500 mb-1">Response</div>
              {call.response_blocks?.map((block, i) => (
                <div key={i} className="mb-2">
                  {block.type === "text" && block.text && (
                    <div className="p-3 bg-gray-800/50 rounded text-xs">
                      <Markdown content={block.text} />
                    </div>
                  )}
                  {block.type === "tool_use" && (
                    <div className="p-3 bg-amber-900/20 border border-amber-800/30 rounded text-xs">
                      <span className="text-amber-400 font-medium">Tool: {block.tool_name}</span>
                      {block.input && (
                        <pre className="mt-1 text-gray-300">{JSON.stringify(block.input, null, 2)}</pre>
                      )}
                    </div>
                  )}
                  {block.type === "thinking" && block.text && (
                    <div className="p-3 bg-purple-900/20 border border-purple-800/30 rounded text-xs text-purple-200 whitespace-pre-wrap">
                      {block.text}
                    </div>
                  )}
                </div>
              )) ?? (
                call.response?.content && (
                  <div className="p-3 bg-gray-800/50 rounded text-xs">
                    <Markdown content={call.response.content} />
                  </div>
                )
              )}
            </div>
          )}
        </div>
      );
    }
    case "tool_call":
      return (
        <div className="space-y-3 text-sm">
          {step.tool_call?.input && (
            <div>
              <div className="text-[10px] uppercase text-gray-600 mb-1">Input</div>
              <pre className="p-3 bg-gray-800/50 rounded text-xs text-gray-300">{JSON.stringify(step.tool_call.input, null, 2)}</pre>
            </div>
          )}
          {step.tool_call?.output != null && (
            <div>
              <div className="text-[10px] uppercase text-gray-600 mb-1">Output</div>
              <div className="p-3 bg-gray-800/50 rounded text-xs">
                <Markdown content={typeof step.tool_call.output === "string" ? step.tool_call.output : JSON.stringify(step.tool_call.output, null, 2)} />
              </div>
            </div>
          )}
        </div>
      );
    case "thought":
      return (
        <div className="p-3 bg-purple-900/20 border border-purple-800/30 rounded text-sm text-purple-200 whitespace-pre-wrap">
          {step.thought?.content}
        </div>
      );
    case "agent_decision":
      return (
        <div className="space-y-3 text-sm">
          <div className="font-medium">{step.agent_decision?.decision}</div>
          {step.agent_decision?.reasoning && (
            <div className="text-gray-400">{step.agent_decision.reasoning}</div>
          )}
          {step.agent_decision?.alternatives_considered && step.agent_decision.alternatives_considered.length > 0 && (
            <div>
              <div className="text-[10px] uppercase text-gray-600 mb-1">Alternatives Considered</div>
              <ul className="list-disc ml-4 text-gray-500 text-xs space-y-1">
                {step.agent_decision.alternatives_considered.map((a, i) => <li key={i}>{a}</li>)}
              </ul>
            </div>
          )}
        </div>
      );
    case "error":
      return (
        <div className="space-y-2 text-sm">
          {step.error?.code && <div className="text-red-400 font-mono text-xs">{step.error.code}</div>}
          <div className="text-red-300">{step.error?.message}</div>
          {step.error?.stack_trace && (
            <pre className="p-3 bg-red-900/10 rounded text-xs text-red-400 whitespace-pre-wrap">{step.error.stack_trace}</pre>
          )}
        </div>
      );
    default:
      return <pre className="text-xs text-gray-400">{JSON.stringify(step, null, 2)}</pre>;
  }
}
