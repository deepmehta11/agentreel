import type { Trajectory } from "./types";

export function parseTrajectory(json: string): Trajectory {
  return JSON.parse(json) as Trajectory;
}

export function formatDuration(ms: number): string {
  if (ms > 60_000) return `${(ms / 60_000).toFixed(1)}m`;
  if (ms > 1_000) return `${(ms / 1_000).toFixed(1)}s`;
  return `${Math.round(ms)}ms`;
}

export function formatTokens(n: number): string {
  if (n > 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n > 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

export function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(2)}`;
}

export function statusIcon(status?: string): string {
  switch (status) {
    case "success":
      return "\u2705";
    case "failure":
      return "\u274C";
    case "partial":
      return "\u26A0\uFE0F";
    case "aborted":
      return "\uD83D\uDED1";
    default:
      return "\u2754";
  }
}

export function stepIcon(type: string): string {
  switch (type) {
    case "llm_call":
      return "\uD83E\uDDE0";
    case "tool_call":
      return "\uD83D\uDD27";
    case "tool_result":
      return "\uD83D\uDCE5";
    case "human_input":
      return "\uD83D\uDC64";
    case "error":
      return "\u274C";
    case "retry":
      return "\uD83D\uDD04";
    case "thought":
      return "\uD83D\uDCAD";
    case "agent_decision":
      return "\uD83D\uDD00";
    case "file_operation":
      return "\uD83D\uDCC1";
    case "screenshot":
      return "\uD83D\uDCF8";
    case "checkpoint":
      return "\uD83D\uDCCC";
    default:
      return "\u25CF";
  }
}
