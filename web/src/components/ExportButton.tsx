"use client";

import type { Trajectory } from "@/lib/types";

export function ExportButton({ trajectory }: { trajectory: Trajectory }) {
  const handleExport = () => {
    const html = generateStandaloneHTML(trajectory);
    const blob = new Blob([html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${trajectory.metadata.title?.replace(/\s+/g, "-").toLowerCase() ?? trajectory.id}.html`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <button
      onClick={handleExport}
      className="text-xs px-3 py-1.5 bg-gray-800 hover:bg-gray-700 rounded border border-gray-700 text-gray-300"
    >
      Export HTML
    </button>
  );
}

function generateStandaloneHTML(t: Trajectory): string {
  const title = t.metadata.title ?? "Trajectory";
  const steps = t.steps;
  const stats = t.stats;

  const stepsHtml = steps
    .map((step) => {
      const icon = {
        llm_call: "\uD83E\uDDE0",
        tool_call: "\uD83D\uDD27",
        tool_result: "\uD83D\uDCE5",
        thought: "\uD83D\uDCAD",
        agent_decision: "\uD83D\uDD00",
        error: "\u274C",
        human_input: "\uD83D\uDC64",
        file_operation: "\uD83D\uDCC1",
        retry: "\uD83D\uDD04",
        checkpoint: "\uD83D\uDCCC",
        screenshot: "\uD83D\uDCF8",
      }[step.type] ?? "\u25CF";

      let content = "";
      const dur = step.duration_ms ? `${(step.duration_ms / 1000).toFixed(1)}s` : "";
      const tokens = step.tokens
        ? `${step.tokens.input_tokens ?? 0}in/${step.tokens.output_tokens ?? 0}out`
        : "";

      switch (step.type) {
        case "llm_call": {
          const model = step.llm_call?.model ?? "unknown";
          const response = step.llm_call?.response?.content ?? "";
          const thinking = step.llm_call?.thinking ?? "";
          content = `<strong>LLM Call (${esc(model)})</strong>`;
          if (step.llm_call?.system_prompt) {
            content += `<div class="meta">System: ${esc(step.llm_call.system_prompt.slice(0, 200))}</div>`;
          }
          step.llm_call?.messages?.forEach((m) => {
            content += `<div class="msg ${m.role}"><span class="role">${esc(m.role)}</span> ${esc(String(m.content ?? "").slice(0, 500))}</div>`;
          });
          if (thinking) {
            content += `<div class="thinking">\uD83D\uDCAD ${esc(thinking.slice(0, 1000))}</div>`;
          }
          step.llm_call?.response_blocks?.forEach((block) => {
            if (block.type === "text" && block.text) {
              content += `<div class="response">${esc(block.text)}</div>`;
            } else if (block.type === "tool_use") {
              content += `<div class="tool-use">\uD83D\uDD27 ${esc(block.tool_name ?? "?")}(${esc(JSON.stringify(block.input ?? {}))})</div>`;
            }
          });
          if (!step.llm_call?.response_blocks?.length && response) {
            content += `<div class="response">${esc(response)}</div>`;
          }
          break;
        }
        case "tool_call":
          content = `<strong>${esc(step.tool_call?.name ?? "tool")}</strong>`;
          if (step.tool_call?.input) content += `<pre>${esc(JSON.stringify(step.tool_call.input, null, 2))}</pre>`;
          break;
        case "thought":
          content = `<div class="thinking">${esc(step.thought?.content ?? "")}</div>`;
          break;
        case "agent_decision":
          content = `<strong>${esc(step.agent_decision?.decision ?? "")}</strong>`;
          if (step.agent_decision?.reasoning) content += `<div class="meta">${esc(step.agent_decision.reasoning)}</div>`;
          break;
        case "error":
          content = `<div class="error-msg">${esc(step.error?.message ?? "")}</div>`;
          break;
        default:
          content = `<strong>${step.type}</strong>`;
      }

      return `<div class="step">
        <div class="step-header">
          <span class="icon">${icon}</span>
          <span class="step-num">#${step.index}</span>
          <span class="step-content">${content}</span>
          <span class="step-meta">${tokens} ${dur}</span>
        </div>
      </div>`;
    })
    .join("\n");

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(title)} — AgentReel</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #0a0a0f; color: #e5e5e5; padding: 2rem; max-width: 900px; margin: 0 auto; }
  h1 { font-size: 1.5rem; margin-bottom: 0.5rem; }
  .meta { color: #888; font-size: 0.75rem; margin-top: 0.25rem; }
  .stats { display: grid; grid-template-columns: repeat(5, 1fr); gap: 1rem; margin: 1.5rem 0; }
  .stat { background: #111; border: 1px solid #222; border-radius: 8px; padding: 0.75rem; }
  .stat-label { font-size: 0.65rem; color: #666; text-transform: uppercase; letter-spacing: 0.05em; }
  .stat-value { font-size: 0.9rem; font-weight: 600; margin-top: 0.25rem; }
  .step { border: 1px solid #1a1a2e; border-radius: 8px; margin-bottom: 0.5rem; overflow: hidden; }
  .step-header { display: flex; align-items: flex-start; gap: 0.75rem; padding: 0.75rem 1rem; }
  .icon { font-size: 1.1rem; }
  .step-num { font-family: monospace; color: #555; font-size: 0.7rem; min-width: 2rem; padding-top: 0.15rem; }
  .step-content { flex: 1; font-size: 0.8rem; }
  .step-meta { color: #555; font-size: 0.7rem; white-space: nowrap; }
  .msg { margin: 0.5rem 0; padding: 0.5rem; background: #0d0d1a; border-radius: 4px; font-size: 0.75rem; white-space: pre-wrap; word-break: break-word; }
  .role { font-weight: 600; text-transform: uppercase; font-size: 0.6rem; }
  .msg.user .role { color: #60a5fa; }
  .msg.assistant .role { color: #4ade80; }
  .msg.system .role { color: #fbbf24; }
  .response { margin: 0.5rem 0; padding: 0.75rem; background: #0d1117; border-radius: 4px; font-size: 0.75rem; white-space: pre-wrap; word-break: break-word; }
  .thinking { margin: 0.5rem 0; padding: 0.75rem; background: #1a0a2e; border: 1px solid #2d1b4e; border-radius: 4px; font-size: 0.75rem; color: #c4b5fd; white-space: pre-wrap; }
  .tool-use { margin: 0.5rem 0; padding: 0.5rem; background: #1a1400; border: 1px solid #3d2e00; border-radius: 4px; font-size: 0.75rem; color: #fbbf24; }
  .error-msg { color: #f87171; }
  pre { background: #0a0a0f; padding: 0.5rem; border-radius: 4px; font-size: 0.7rem; overflow-x: auto; margin-top: 0.5rem; }
  .badge { display: inline-block; background: #1a1a2e; padding: 0.15rem 0.5rem; border-radius: 4px; font-size: 0.7rem; margin-right: 0.25rem; }
  .footer { margin-top: 2rem; padding-top: 1rem; border-top: 1px solid #1a1a2e; font-size: 0.7rem; color: #444; text-align: center; }
</style>
</head>
<body>
<h1>${esc(title)}</h1>
<div class="meta">ID: ${t.id} · ${t.metadata.created_at}</div>
${t.metadata.tags?.length ? `<div style="margin-top:0.5rem">${t.metadata.tags.map((tag) => `<span class="badge">${esc(tag)}</span>`).join("")}</div>` : ""}

<div class="stats">
  <div class="stat"><div class="stat-label">Steps</div><div class="stat-value">${stats?.total_steps ?? steps.length}</div></div>
  <div class="stat"><div class="stat-label">LLM / Tool</div><div class="stat-value">${stats?.total_llm_calls ?? "?"} / ${stats?.total_tool_calls ?? "?"}</div></div>
  <div class="stat"><div class="stat-label">Tokens</div><div class="stat-value">${stats?.total_tokens ? ((stats.total_tokens.input_tokens ?? 0) + (stats.total_tokens.output_tokens ?? 0)).toLocaleString() : "?"}</div></div>
  <div class="stat"><div class="stat-label">Cost</div><div class="stat-value" style="color:#4ade80">${stats?.total_cost_usd != null ? `$${stats.total_cost_usd.toFixed(4)}` : "?"}</div></div>
  <div class="stat"><div class="stat-label">Duration</div><div class="stat-value">${stats?.total_duration_ms != null ? `${(stats.total_duration_ms / 1000).toFixed(1)}s` : "?"}</div></div>
</div>

<h2 style="font-size:1rem;margin-bottom:0.75rem">Steps</h2>
${stepsHtml}

${t.outcome ? `<div style="margin-top:1.5rem;padding:1rem;border:1px solid #1a1a2e;border-radius:8px">
  <strong>${t.outcome.status === "success" ? "\u2705" : "\u274C"} ${t.outcome.status}</strong>
  ${t.outcome.summary ? `<div class="meta">${esc(t.outcome.summary)}</div>` : ""}
</div>` : ""}

<div class="footer">Generated by AgentReel · ${new Date().toISOString()}</div>
</body>
</html>`;
}

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
