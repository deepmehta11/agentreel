"use client";

import type { Trajectory, Step } from "@/lib/types";

interface Insight {
  type: "warning" | "info" | "optimization" | "error";
  title: string;
  description: string;
  stepIndexes?: number[];
}

export function SmartAnalysis({ trajectory }: { trajectory: Trajectory }) {
  const insights = analyzeTrajectory(trajectory);

  if (insights.length === 0) {
    return (
      <div className="border border-gray-800 rounded-lg p-4 text-center text-sm text-gray-600">
        No issues detected. This trajectory looks clean.
      </div>
    );
  }

  return (
    <div className="border border-gray-800 rounded-lg overflow-hidden">
      <div className="px-4 py-2 bg-gray-900 border-b border-gray-800">
        <span className="text-xs text-gray-500 font-medium uppercase tracking-wider">
          Smart Analysis — {insights.length} insight{insights.length !== 1 ? "s" : ""}
        </span>
      </div>
      <div className="divide-y divide-gray-800/50">
        {insights.map((insight, i) => (
          <div key={i} className="px-4 py-3 flex gap-3">
            <span className="text-lg shrink-0">
              {insight.type === "warning" ? "\u26A0\uFE0F" :
               insight.type === "error" ? "\u274C" :
               insight.type === "optimization" ? "\uD83D\uDCA1" : "\u2139\uFE0F"}
            </span>
            <div>
              <div className="text-sm font-medium text-gray-200">{insight.title}</div>
              <div className="text-xs text-gray-500 mt-0.5">{insight.description}</div>
              {insight.stepIndexes && insight.stepIndexes.length > 0 && (
                <div className="text-xs text-gray-600 mt-1">
                  Steps: {insight.stepIndexes.map((i) => `#${i}`).join(", ")}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function analyzeTrajectory(trajectory: Trajectory): Insight[] {
  const insights: Insight[] = [];
  const steps = trajectory.steps;

  // 1. Detect retry loops (same tool called multiple times in a row)
  detectRetryLoops(steps, insights);

  // 2. Detect high token waste (large input, tiny output)
  detectTokenWaste(steps, insights);

  // 3. Detect long pauses between steps
  detectLongPauses(steps, insights);

  // 4. Detect errors followed by retries
  detectErrorRecovery(steps, insights);

  // 5. Cost concentration (one step costs >50% of total)
  detectCostConcentration(steps, trajectory.stats?.total_cost_usd ?? 0, insights);

  // 6. Thinking token ratio
  detectThinkingOverhead(steps, insights);

  // 7. Tool call without result
  detectOrphanToolCalls(steps, insights);

  // 8. Overall health summary
  if (trajectory.outcome?.status === "success" && insights.filter((i) => i.type === "error" || i.type === "warning").length === 0) {
    insights.push({
      type: "info",
      title: "Clean run",
      description: `Completed successfully in ${steps.length} steps with no issues detected.`,
    });
  }

  return insights;
}

function detectRetryLoops(steps: Step[], insights: Insight[]) {
  let prev = "";
  let count = 0;
  let startIdx = 0;
  const retrySteps: number[] = [];

  for (const step of steps) {
    const sig = stepSignature(step);
    if (sig === prev && sig !== "") {
      count++;
      retrySteps.push(step.index);
    } else {
      if (count >= 2) {
        insights.push({
          type: "warning",
          title: `Retry loop detected (${count + 1} repetitions)`,
          description: `The same action "${prev}" was repeated ${count + 1} times consecutively. This might indicate the agent is stuck.`,
          stepIndexes: retrySteps.slice(),
        });
      }
      prev = sig;
      count = 0;
      retrySteps.length = 0;
      retrySteps.push(step.index);
      startIdx = step.index;
    }
  }
}

function detectTokenWaste(steps: Step[], insights: Insight[]) {
  for (const step of steps) {
    if (step.type === "llm_call" && step.tokens) {
      const input = step.tokens.input_tokens ?? 0;
      const output = step.tokens.output_tokens ?? 0;
      if (input > 5000 && output < 50) {
        insights.push({
          type: "optimization",
          title: `High input/output ratio at step #${step.index}`,
          description: `Sent ${input.toLocaleString()} tokens but got only ${output} back. Consider trimming context or using a cheaper model for this call.`,
          stepIndexes: [step.index],
        });
      }
    }
  }
}

function detectLongPauses(steps: Step[], insights: Insight[]) {
  for (const step of steps) {
    if (step.duration_ms && step.duration_ms > 30000) {
      insights.push({
        type: "warning",
        title: `Slow step #${step.index} (${(step.duration_ms / 1000).toFixed(1)}s)`,
        description: `This step took over 30 seconds. Check if the API was rate-limited or the model was overloaded.`,
        stepIndexes: [step.index],
      });
    }
  }
}

function detectErrorRecovery(steps: Step[], insights: Insight[]) {
  const errors = steps.filter((s) => s.type === "error");
  if (errors.length > 0) {
    const errorAfterRecovery = errors.filter((e) => {
      const nextSteps = steps.filter((s) => s.index > e.index && s.index <= e.index + 3);
      return nextSteps.some((s) => s.type !== "error");
    });

    if (errorAfterRecovery.length > 0 && errorAfterRecovery.length === errors.length) {
      insights.push({
        type: "info",
        title: `${errors.length} error(s) recovered`,
        description: `The agent hit ${errors.length} error(s) but recovered from all of them.`,
        stepIndexes: errors.map((e) => e.index),
      });
    } else {
      const unrecovered = errors.length - errorAfterRecovery.length;
      if (unrecovered > 0) {
        insights.push({
          type: "error",
          title: `${unrecovered} unrecovered error(s)`,
          description: `The agent failed to recover from ${unrecovered} error(s).`,
          stepIndexes: errors.map((e) => e.index),
        });
      }
    }
  }
}

function detectCostConcentration(steps: Step[], totalCost: number, insights: Insight[]) {
  if (totalCost <= 0) return;
  for (const step of steps) {
    const cost = step.cost_usd ?? 0;
    if (cost > totalCost * 0.5 && steps.length > 2) {
      insights.push({
        type: "optimization",
        title: `Step #${step.index} accounts for ${((cost / totalCost) * 100).toFixed(0)}% of total cost`,
        description: `This single step cost $${cost.toFixed(4)} out of $${totalCost.toFixed(4)} total. Consider using a cheaper model or reducing context for this call.`,
        stepIndexes: [step.index],
      });
    }
  }
}

function detectThinkingOverhead(steps: Step[], insights: Insight[]) {
  for (const step of steps) {
    if (step.tokens?.thinking_tokens && step.tokens.thinking_tokens > 0) {
      const total = (step.tokens.input_tokens ?? 0) + (step.tokens.output_tokens ?? 0) + step.tokens.thinking_tokens;
      const thinkingPct = (step.tokens.thinking_tokens / total) * 100;
      if (thinkingPct > 70) {
        insights.push({
          type: "info",
          title: `Heavy reasoning at step #${step.index} (${thinkingPct.toFixed(0)}% thinking tokens)`,
          description: `${step.tokens.thinking_tokens.toLocaleString()} thinking tokens used. This is normal for complex reasoning tasks but consider if extended thinking is needed.`,
          stepIndexes: [step.index],
        });
      }
    }
  }
}

function detectOrphanToolCalls(steps: Step[], insights: Insight[]) {
  const toolCallIndexes = steps
    .filter((s) => s.type === "tool_call")
    .map((s) => s.index);
  const toolResultParents = steps
    .filter((s) => s.type === "tool_result" && s.parent_step_id != null)
    .map((s) => s.parent_step_id!);

  const orphans = toolCallIndexes.filter((i) => !toolResultParents.includes(i));
  // Only flag if there are tool_result steps (otherwise the trajectory format just doesn't use them)
  const hasAnyToolResults = steps.some((s) => s.type === "tool_result");
  if (orphans.length > 0 && hasAnyToolResults) {
    insights.push({
      type: "warning",
      title: `${orphans.length} tool call(s) without results`,
      description: `Tool calls at steps ${orphans.map((i) => `#${i}`).join(", ")} have no matching tool_result. The results may have been lost.`,
      stepIndexes: orphans,
    });
  }
}

function stepSignature(step: Step): string {
  switch (step.type) {
    case "llm_call":
      return `llm:${step.llm_call?.model ?? ""}`;
    case "tool_call":
      return `tool:${step.tool_call?.name ?? ""}:${JSON.stringify(step.tool_call?.input ?? "")}`;
    case "error":
      return `error:${step.error?.message ?? ""}`;
    default:
      return "";
  }
}
