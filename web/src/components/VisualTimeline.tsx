"use client";

import type { Step } from "@/lib/types";
import { formatDuration, stepIcon } from "@/lib/parse";
import { useState } from "react";

const LANE_COLORS: Record<string, { bg: string; border: string; text: string }> = {
  llm_call: { bg: "bg-blue-900/30", border: "border-blue-700/50", text: "text-blue-300" },
  tool_call: { bg: "bg-amber-900/30", border: "border-amber-700/50", text: "text-amber-300" },
  tool_result: { bg: "bg-amber-900/20", border: "border-amber-700/30", text: "text-amber-400" },
  thinking: { bg: "bg-purple-900/30", border: "border-purple-700/50", text: "text-purple-300" },
  thought: { bg: "bg-purple-900/30", border: "border-purple-700/50", text: "text-purple-300" },
  agent_decision: { bg: "bg-teal-900/30", border: "border-teal-700/50", text: "text-teal-300" },
  error: { bg: "bg-red-900/30", border: "border-red-700/50", text: "text-red-300" },
  human_input: { bg: "bg-green-900/30", border: "border-green-700/50", text: "text-green-300" },
  file_operation: { bg: "bg-gray-800/50", border: "border-gray-700/50", text: "text-gray-300" },
  default: { bg: "bg-gray-800/30", border: "border-gray-700/50", text: "text-gray-400" },
};

function getColor(type: string) {
  return LANE_COLORS[type] ?? LANE_COLORS.default;
}

export function VisualTimeline({
  steps,
  onSelectStep,
}: {
  steps: Step[];
  onSelectStep: (index: number) => void;
}) {
  const [hoveredStep, setHoveredStep] = useState<number | null>(null);

  if (steps.length === 0) return null;

  // Calculate time range
  const firstTime = new Date(steps[0].timestamp).getTime();
  const totalDuration = steps.reduce((sum, s) => sum + (s.duration_ms ?? 0), 0);
  const maxTime = totalDuration > 0 ? totalDuration : 10000;

  // Assign lanes
  const lanes = assignLanes(steps);
  const laneNames = Array.from(new Set(lanes.map((l) => l.lane)));

  // Running time offset for each step
  let runningTime = 0;
  const stepPositions = steps.map((step) => {
    const start = runningTime;
    const dur = step.duration_ms ?? 100;
    runningTime += dur;
    return { start, duration: dur, end: start + dur };
  });

  return (
    <div className="border border-gray-800 rounded-lg overflow-hidden">
      {/* Header */}
      <div className="px-4 py-2 bg-gray-900 border-b border-gray-800 flex items-center justify-between">
        <span className="text-xs text-gray-500 font-medium uppercase tracking-wider">Timeline</span>
        <span className="text-xs text-gray-600">{formatDuration(maxTime)} total</span>
      </div>

      {/* Lanes */}
      <div className="relative px-4 py-3">
        {/* Time axis */}
        <div className="flex justify-between text-[10px] text-gray-600 mb-2 px-1">
          <span>0s</span>
          <span>{formatDuration(maxTime * 0.25)}</span>
          <span>{formatDuration(maxTime * 0.5)}</span>
          <span>{formatDuration(maxTime * 0.75)}</span>
          <span>{formatDuration(maxTime)}</span>
        </div>

        {/* Grid lines */}
        <div className="absolute inset-x-4 top-10 bottom-3 pointer-events-none">
          {[0.25, 0.5, 0.75].map((pct) => (
            <div
              key={pct}
              className="absolute top-0 bottom-0 border-l border-gray-800/50"
              style={{ left: `${pct * 100}%` }}
            />
          ))}
        </div>

        {/* Swim lanes */}
        {laneNames.map((laneName) => {
          const laneSteps = lanes.filter((l) => l.lane === laneName);
          const color = getColor(laneName);

          return (
            <div key={laneName} className="flex items-center gap-2 mb-1.5">
              {/* Lane label */}
              <div className="w-20 shrink-0 text-right">
                <span className={`text-[10px] ${color.text}`}>{laneName.replace("_", " ")}</span>
              </div>

              {/* Lane bar */}
              <div className="flex-1 relative h-7 bg-gray-900/30 rounded">
                {laneSteps.map(({ step, originalIndex }) => {
                  const pos = stepPositions[originalIndex];
                  if (!pos) return null;
                  const leftPct = (pos.start / maxTime) * 100;
                  const widthPct = Math.max((pos.duration / maxTime) * 100, 0.5);
                  const isHovered = hoveredStep === originalIndex;

                  return (
                    <button
                      key={step.index}
                      className={`absolute top-0.5 bottom-0.5 rounded transition-all cursor-pointer border ${color.bg} ${color.border} ${
                        isHovered ? "ring-1 ring-white/30 z-10" : ""
                      }`}
                      style={{
                        left: `${leftPct}%`,
                        width: `${widthPct}%`,
                        minWidth: "6px",
                      }}
                      onMouseEnter={() => setHoveredStep(originalIndex)}
                      onMouseLeave={() => setHoveredStep(null)}
                      onClick={() => onSelectStep(originalIndex)}
                      title={`#${step.index} ${stepLabel(step)} — ${formatDuration(pos.duration)}`}
                    >
                      {widthPct > 3 && (
                        <span className={`text-[9px] px-1 truncate block ${color.text}`}>
                          {stepIcon(step.type)} {shortLabel(step)}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
          );
        })}

        {/* Connecting lines for parent_step_id */}
        {/* Rendered as visual hints in the lane bars above */}
      </div>

      {/* Tooltip */}
      {hoveredStep !== null && steps[hoveredStep] && (
        <div className="px-4 py-2 border-t border-gray-800 bg-gray-900/80 flex items-center gap-3 text-xs">
          <span className="text-lg">{stepIcon(steps[hoveredStep].type)}</span>
          <span className="font-mono text-gray-500">#{steps[hoveredStep].index}</span>
          <span className="text-gray-200">{stepLabel(steps[hoveredStep])}</span>
          {steps[hoveredStep].duration_ms && (
            <span className="text-gray-500">{formatDuration(steps[hoveredStep].duration_ms!)}</span>
          )}
          {steps[hoveredStep].tokens && (
            <span className="text-gray-500">
              {steps[hoveredStep].tokens!.input_tokens ?? 0}/{steps[hoveredStep].tokens!.output_tokens ?? 0} tokens
            </span>
          )}
          {steps[hoveredStep].cost_usd != null && steps[hoveredStep].cost_usd! > 0 && (
            <span className="text-green-400">${steps[hoveredStep].cost_usd!.toFixed(4)}</span>
          )}
          <span className="text-gray-600 ml-auto">Click to expand</span>
        </div>
      )}
    </div>
  );
}

function assignLanes(steps: Step[]): Array<{ step: Step; lane: string; originalIndex: number }> {
  return steps.map((step, i) => {
    let lane = step.type;
    // Group thinking with thought
    if (step.type === "llm_call" && step.llm_call?.thinking) {
      // Still an LLM call lane, thinking shown inside
    }
    return { step, lane, originalIndex: i };
  });
}

function stepLabel(step: Step): string {
  switch (step.type) {
    case "llm_call":
      return `LLM call (${step.llm_call?.model ?? "?"})`;
    case "tool_call":
      return step.tool_call?.name ?? "tool";
    case "tool_result":
      return `Result: ${step.tool_result?.tool_name ?? "?"}`;
    case "thought":
      return step.thought?.content?.slice(0, 60) ?? "thinking...";
    case "agent_decision":
      return step.agent_decision?.decision?.slice(0, 60) ?? "decision";
    case "error":
      return step.error?.message ?? "error";
    case "human_input":
      return `Human ${step.human_input?.action ?? "input"}`;
    case "file_operation":
      return `${step.file_operation?.operation ?? "file"} ${step.file_operation?.path ?? ""}`;
    default:
      return step.type;
  }
}

function shortLabel(step: Step): string {
  switch (step.type) {
    case "llm_call":
      return step.llm_call?.model?.split("-").pop() ?? "llm";
    case "tool_call":
      return step.tool_call?.name ?? "tool";
    case "tool_result":
      return step.tool_result?.tool_name ?? "result";
    case "thought":
      return "think";
    case "agent_decision":
      return "decide";
    case "error":
      return "err";
    default:
      return step.type.slice(0, 6);
  }
}
