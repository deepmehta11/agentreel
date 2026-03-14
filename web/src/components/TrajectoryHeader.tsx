import type { Trajectory } from "@/lib/types";
import { formatDuration, formatTokens, formatCost, statusIcon } from "@/lib/parse";

export function TrajectoryHeader({ trajectory }: { trajectory: Trajectory }) {
  const { metadata, stats, outcome } = trajectory;
  const model = metadata.model?.model_id ?? "unknown";
  const provider = metadata.model?.provider ?? "";
  const agent = metadata.agent?.name ?? "";
  const status = outcome?.status;

  return (
    <div className="border border-gray-800 rounded-lg p-6 space-y-4">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold">
            {metadata.title ?? "Untitled Run"}
          </h1>
          {metadata.description && (
            <p className="text-gray-400 mt-1">{metadata.description}</p>
          )}
        </div>
        {status && (
          <span className="text-2xl">{statusIcon(status)}</span>
        )}
      </div>

      <div className="flex flex-wrap gap-4 text-sm text-gray-400">
        {provider && model && (
          <span>
            <span className="text-gray-600">Model:</span>{" "}
            {provider}/{model}
          </span>
        )}
        {agent && (
          <span>
            <span className="text-gray-600">Agent:</span> {agent}
          </span>
        )}
        <span>
          <span className="text-gray-600">ID:</span>{" "}
          <span className="font-mono text-xs">{trajectory.id}</span>
        </span>
        {trajectory.parent_id && (
          <span>
            <span className="text-gray-600">Forked from:</span>{" "}
            <span className="font-mono text-xs">{trajectory.parent_id}</span>
          </span>
        )}
      </div>

      {metadata.tags && metadata.tags.length > 0 && (
        <div className="flex gap-2">
          {metadata.tags.map((tag) => (
            <span
              key={tag}
              className="px-2 py-1 text-xs rounded bg-gray-800 text-gray-300"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4 pt-2 border-t border-gray-800">
          <Stat label="Steps" value={String(stats.total_steps)} />
          <Stat
            label="LLM / Tool"
            value={`${stats.total_llm_calls} / ${stats.total_tool_calls}`}
          />
          <Stat
            label="Tokens"
            value={
              stats.total_tokens
                ? formatTokens(
                    (stats.total_tokens.input_tokens ?? 0) +
                      (stats.total_tokens.output_tokens ?? 0)
                  )
                : "-"
            }
          />
          <Stat
            label="Cost"
            value={
              stats.total_cost_usd != null
                ? formatCost(stats.total_cost_usd)
                : "-"
            }
          />
          <Stat
            label="Duration"
            value={
              stats.total_duration_ms != null
                ? formatDuration(stats.total_duration_ms)
                : "-"
            }
          />
        </div>
      )}

      {outcome?.summary && (
        <div className="pt-2 border-t border-gray-800">
          <p className="text-sm text-gray-300">{outcome.summary}</p>
        </div>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-xs text-gray-600">{label}</div>
      <div className="text-sm font-medium">{value}</div>
    </div>
  );
}
