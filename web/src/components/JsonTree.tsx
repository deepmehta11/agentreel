"use client";

import { useState } from "react";

/** Collapsible JSON tree viewer — like Firefox DevTools JSON panel. */
export function JsonTree({ data, defaultExpanded = 1 }: { data: unknown; defaultExpanded?: number }) {
  return (
    <div className="font-mono text-xs">
      <JsonNode value={data} depth={0} defaultExpanded={defaultExpanded} keyName={undefined} />
    </div>
  );
}

function JsonNode({
  keyName,
  value,
  depth,
  defaultExpanded,
}: {
  keyName: string | undefined;
  value: unknown;
  depth: number;
  defaultExpanded: number;
}) {
  const [expanded, setExpanded] = useState(depth < defaultExpanded);

  if (value === null) return <Leaf keyName={keyName} value="null" color="text-gray-500" />;
  if (value === undefined) return <Leaf keyName={keyName} value="undefined" color="text-gray-500" />;
  if (typeof value === "boolean") return <Leaf keyName={keyName} value={String(value)} color="text-purple-400" />;
  if (typeof value === "number") return <Leaf keyName={keyName} value={String(value)} color="text-blue-400" />;
  if (typeof value === "string") {
    if (value.length > 200 && !expanded) {
      return (
        <div className="flex" style={{ paddingLeft: depth * 16 }}>
          {keyName !== undefined && <span className="text-cyan-400">{`"${keyName}"`}: </span>}
          <span className="text-green-400">"{value.slice(0, 200)}</span>
          <button onClick={() => setExpanded(true)} className="text-brand-400 hover:text-brand-300 ml-1">
            ...{value.length - 200} more
          </button>
          <span className="text-green-400">"</span>
        </div>
      );
    }
    return <Leaf keyName={keyName} value={`"${value}"`} color="text-green-400" depth={depth} />;
  }

  if (Array.isArray(value)) {
    if (value.length === 0) return <Leaf keyName={keyName} value="[]" color="text-gray-500" depth={depth} />;

    return (
      <div style={{ paddingLeft: depth > 0 ? 16 : 0 }}>
        <button onClick={() => setExpanded(!expanded)} className="flex items-center hover:bg-gray-800/50 -ml-4 pl-4 w-full text-left">
          <span className="text-gray-600 w-3 mr-1">{expanded ? "▼" : "▶"}</span>
          {keyName !== undefined && <span className="text-cyan-400">{`"${keyName}"`}: </span>}
          <span className="text-gray-500">{expanded ? "[" : `Array(${value.length})`}</span>
        </button>
        {expanded && (
          <>
            {value.map((item, i) => (
              <JsonNode key={i} keyName={String(i)} value={item} depth={depth + 1} defaultExpanded={defaultExpanded} />
            ))}
            <div style={{ paddingLeft: 0 }} className="text-gray-500">]</div>
          </>
        )}
      </div>
    );
  }

  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return <Leaf keyName={keyName} value="{}" color="text-gray-500" depth={depth} />;

    return (
      <div style={{ paddingLeft: depth > 0 ? 16 : 0 }}>
        <button onClick={() => setExpanded(!expanded)} className="flex items-center hover:bg-gray-800/50 -ml-4 pl-4 w-full text-left">
          <span className="text-gray-600 w-3 mr-1">{expanded ? "▼" : "▶"}</span>
          {keyName !== undefined && <span className="text-cyan-400">{`"${keyName}"`}: </span>}
          <span className="text-gray-500">{expanded ? "{" : `{${entries.length} keys}`}</span>
        </button>
        {expanded && (
          <>
            {entries.map(([k, v]) => (
              <JsonNode key={k} keyName={k} value={v} depth={depth + 1} defaultExpanded={defaultExpanded} />
            ))}
            <div style={{ paddingLeft: 0 }} className="text-gray-500">{"}"}</div>
          </>
        )}
      </div>
    );
  }

  return <Leaf keyName={keyName} value={String(value)} color="text-gray-300" depth={depth} />;
}

function Leaf({
  keyName,
  value,
  color,
  depth = 0,
}: {
  keyName: string | undefined;
  value: string;
  color: string;
  depth?: number;
}) {
  return (
    <div style={{ paddingLeft: depth * 16 }} className="leading-5">
      {keyName !== undefined && <span className="text-cyan-400">{`"${keyName}"`}: </span>}
      <span className={`${color} whitespace-pre-wrap break-all`}>{value}</span>
    </div>
  );
}
