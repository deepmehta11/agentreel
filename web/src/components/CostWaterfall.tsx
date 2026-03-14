"use client";

import type { Step } from "@/lib/types";
import { formatDuration, stepIcon } from "@/lib/parse";

export function CostWaterfall({ steps }: { steps: Step[] }) {
  // Calculate cumulative cost and per-step breakdown
  let cumCost = 0;
  let cumTokens = 0;
  const bars = steps
    .filter((s) => s.type === "llm_call" && (s.cost_usd ?? 0) > 0)
    .map((step) => {
      const cost = step.cost_usd ?? 0;
      const tokens = (step.tokens?.input_tokens ?? 0) + (step.tokens?.output_tokens ?? 0);
      const startCost = cumCost;
      cumCost += cost;
      cumTokens += tokens;
      return {
        step,
        cost,
        tokens,
        cumCost,
        startCost,
        inputTokens: step.tokens?.input_tokens ?? 0,
        outputTokens: step.tokens?.output_tokens ?? 0,
      };
    });

  if (bars.length === 0) {
    return (
      <div className="border border-gray-800 rounded-lg p-6 text-center text-gray-600 text-sm">
        No cost data available. Cost is estimated automatically when recording via the proxy.
      </div>
    );
  }

  const maxCost = cumCost;

  return (
    <div className="border border-gray-800 rounded-lg overflow-hidden">
      <div className="px-4 py-2 bg-gray-900 border-b border-gray-800 flex items-center justify-between">
        <span className="text-xs text-gray-500 font-medium uppercase tracking-wider">Cost Breakdown</span>
        <div className="flex gap-4 text-xs">
          <span className="text-gray-400">Total: <span className="text-green-400 font-medium">${maxCost.toFixed(4)}</span></span>
          <span className="text-gray-400">Tokens: <span className="text-blue-400 font-medium">{cumTokens.toLocaleString()}</span></span>
        </div>
      </div>

      <div className="px-4 py-3 space-y-1.5">
        {bars.map((bar) => {
          const widthPct = maxCost > 0 ? (bar.cost / maxCost) * 100 : 0;
          const leftPct = maxCost > 0 ? (bar.startCost / maxCost) * 100 : 0;

          return (
            <div key={bar.step.index} className="flex items-center gap-3 group">
              {/* Label */}
              <div className="w-48 shrink-0 flex items-center gap-2 text-xs">
                <span>{stepIcon(bar.step.type)}</span>
                <span className="text-gray-400 truncate">
                  #{bar.step.index} {bar.step.llm_call?.model ?? "llm"}
                </span>
              </div>

              {/* Bar */}
              <div className="flex-1 relative h-6 bg-gray-900/50 rounded">
                {/* Cumulative background */}
                <div
                  className="absolute top-0 bottom-0 left-0 bg-green-900/20 rounded-l"
                  style={{ width: `${leftPct + widthPct}%` }}
                />
                {/* This step's cost */}
                <div
                  className="absolute top-0.5 bottom-0.5 bg-green-600/60 rounded transition-all group-hover:bg-green-500/70"
                  style={{ left: `${leftPct}%`, width: `${Math.max(widthPct, 0.5)}%` }}
                />
              </div>

              {/* Cost label */}
              <div className="w-24 shrink-0 text-right text-xs">
                <span className="text-green-400">${bar.cost.toFixed(4)}</span>
              </div>

              {/* Tokens breakdown */}
              <div className="w-32 shrink-0 text-right text-[10px] text-gray-600">
                {bar.inputTokens}in / {bar.outputTokens}out
              </div>
            </div>
          );
        })}
      </div>

      {/* Summary footer */}
      <div className="px-4 py-2 border-t border-gray-800 bg-gray-900/50 flex justify-between text-xs text-gray-500">
        <span>{bars.length} LLM calls</span>
        <span>
          Avg ${(maxCost / bars.length).toFixed(4)}/call ·{" "}
          {Math.round(cumTokens / bars.length)} tokens/call
        </span>
      </div>
    </div>
  );
}
