"use client";

import { useState } from "react";
import type { Trajectory } from "@/lib/types";
import { UploadZone } from "@/components/UploadZone";
import { TrajectoryHeader } from "@/components/TrajectoryHeader";
import { formatDuration, formatTokens, formatCost, stepIcon } from "@/lib/parse";

export default function ComparePage() {
  const [left, setLeft] = useState<Trajectory | null>(null);
  const [right, setRight] = useState<Trajectory | null>(null);

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Compare Trajectories</h1>

      <div className="grid grid-cols-2 gap-6 mb-8">
        <div>
          <h3 className="text-sm text-gray-500 mb-2">Left</h3>
          {left ? (
            <div>
              <TrajectoryHeader trajectory={left} />
              <button
                onClick={() => setLeft(null)}
                className="text-xs text-gray-500 mt-2 hover:text-white"
              >
                Replace
              </button>
            </div>
          ) : (
            <UploadZone onLoad={setLeft} />
          )}
        </div>
        <div>
          <h3 className="text-sm text-gray-500 mb-2">Right</h3>
          {right ? (
            <div>
              <TrajectoryHeader trajectory={right} />
              <button
                onClick={() => setRight(null)}
                className="text-xs text-gray-500 mt-2 hover:text-white"
              >
                Replace
              </button>
            </div>
          ) : (
            <UploadZone onLoad={setRight} />
          )}
        </div>
      </div>

      {left && right && <DiffView left={left} right={right} />}
    </div>
  );
}

function DiffView({ left, right }: { left: Trajectory; right: Trajectory }) {
  const lStats = left.stats;
  const rStats = right.stats;

  return (
    <div className="space-y-6">
      <h2 className="text-lg font-semibold">Comparison</h2>

      {/* Stats comparison */}
      <div className="border border-gray-800 rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-gray-500">
              <th className="px-4 py-2 text-left">Metric</th>
              <th className="px-4 py-2 text-right">Left</th>
              <th className="px-4 py-2 text-right">Right</th>
              <th className="px-4 py-2 text-right">Delta</th>
            </tr>
          </thead>
          <tbody>
            <CompareRow
              label="Steps"
              left={lStats?.total_steps ?? 0}
              right={rStats?.total_steps ?? 0}
            />
            <CompareRow
              label="LLM Calls"
              left={lStats?.total_llm_calls ?? 0}
              right={rStats?.total_llm_calls ?? 0}
            />
            <CompareRow
              label="Tool Calls"
              left={lStats?.total_tool_calls ?? 0}
              right={rStats?.total_tool_calls ?? 0}
            />
            <CompareRow
              label="Tokens"
              left={
                (lStats?.total_tokens?.input_tokens ?? 0) +
                (lStats?.total_tokens?.output_tokens ?? 0)
              }
              right={
                (rStats?.total_tokens?.input_tokens ?? 0) +
                (rStats?.total_tokens?.output_tokens ?? 0)
              }
              format={formatTokens}
            />
            <CompareRow
              label="Cost"
              left={lStats?.total_cost_usd ?? 0}
              right={rStats?.total_cost_usd ?? 0}
              format={formatCost}
            />
            <CompareRow
              label="Duration"
              left={lStats?.total_duration_ms ?? 0}
              right={rStats?.total_duration_ms ?? 0}
              format={formatDuration}
            />
            <CompareRow
              label="Errors"
              left={lStats?.errors_count ?? 0}
              right={rStats?.errors_count ?? 0}
            />
          </tbody>
        </table>
      </div>

      {/* Step-by-step comparison */}
      <h2 className="text-lg font-semibold">Steps</h2>
      <div className="border border-gray-800 rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-gray-500">
              <th className="px-4 py-2 text-left w-8">#</th>
              <th className="px-4 py-2 text-left">Left</th>
              <th className="px-4 py-2 text-left">Right</th>
            </tr>
          </thead>
          <tbody>
            {Array.from({
              length: Math.max(left.steps.length, right.steps.length),
            }).map((_, i) => {
              const lStep = left.steps[i];
              const rStep = right.steps[i];
              const same =
                lStep &&
                rStep &&
                lStep.type === rStep.type &&
                stepLabel(lStep) === stepLabel(rStep);

              return (
                <tr
                  key={i}
                  className={`border-b border-gray-800/50 ${
                    same ? "" : "bg-yellow-500/5"
                  }`}
                >
                  <td className="px-4 py-2 text-gray-600 font-mono text-xs">
                    {i}
                  </td>
                  <td className="px-4 py-2">
                    {lStep ? (
                      <span>
                        {stepIcon(lStep.type)} {stepLabel(lStep)}
                        {lStep.duration_ms && (
                          <span className="text-gray-600 ml-2">
                            {formatDuration(lStep.duration_ms)}
                          </span>
                        )}
                      </span>
                    ) : (
                      <span className="text-gray-700">-</span>
                    )}
                  </td>
                  <td className="px-4 py-2">
                    {rStep ? (
                      <span>
                        {stepIcon(rStep.type)} {stepLabel(rStep)}
                        {rStep.duration_ms && (
                          <span className="text-gray-600 ml-2">
                            {formatDuration(rStep.duration_ms)}
                          </span>
                        )}
                      </span>
                    ) : (
                      <span className="text-gray-700">-</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function stepLabel(step: Trajectory["steps"][0]): string {
  switch (step.type) {
    case "llm_call":
      return step.llm_call?.model ?? "LLM call";
    case "tool_call":
      return step.tool_call?.name ?? "tool";
    case "human_input":
      return step.human_input?.action ?? "input";
    case "error":
      return step.error?.message ?? "error";
    default:
      return step.type;
  }
}

function CompareRow({
  label,
  left,
  right,
  format,
}: {
  label: string;
  left: number;
  right: number;
  format?: (n: number) => string;
}) {
  const fmt = format ?? String;
  const delta = right - left;
  const deltaStr =
    delta === 0
      ? "-"
      : delta > 0
        ? `+${fmt(delta)}`
        : `-${fmt(Math.abs(delta))}`;
  const deltaColor =
    delta === 0
      ? "text-gray-600"
      : delta > 0
        ? "text-red-400"
        : "text-green-400";

  return (
    <tr className="border-b border-gray-800/50">
      <td className="px-4 py-2 text-gray-400">{label}</td>
      <td className="px-4 py-2 text-right font-mono">{fmt(left)}</td>
      <td className="px-4 py-2 text-right font-mono">{fmt(right)}</td>
      <td className={`px-4 py-2 text-right font-mono ${deltaColor}`}>
        {deltaStr}
      </td>
    </tr>
  );
}
