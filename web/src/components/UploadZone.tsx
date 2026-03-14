"use client";

import { useCallback, useState } from "react";
import type { Trajectory } from "@/lib/types";

export function UploadZone({
  onLoad,
}: {
  onLoad: (trajectory: Trajectory) => void;
}) {
  const [dragOver, setDragOver] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleFile = useCallback(
    (file: File) => {
      setError(null);
      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const data = JSON.parse(e.target?.result as string);
          if (!data.version || !data.steps) {
            setError("Not a valid trajectory file (missing version or steps)");
            return;
          }
          onLoad(data as Trajectory);
        } catch {
          setError("Failed to parse JSON file");
        }
      };
      reader.readAsText(file);
    },
    [onLoad]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      const file = e.dataTransfer.files[0];
      if (file) handleFile(file);
    },
    [handleFile]
  );

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={handleDrop}
      className={`border-2 border-dashed rounded-lg p-12 text-center transition-colors ${
        dragOver
          ? "border-brand-400 bg-brand-400/5"
          : "border-gray-700 hover:border-gray-600"
      }`}
    >
      <p className="text-gray-400 mb-2">
        Drop a <code className="text-brand-400">.trajectory.json</code> file
        here
      </p>
      <p className="text-gray-600 text-sm mb-4">or</p>
      <label className="cursor-pointer px-4 py-2 bg-brand-600 hover:bg-brand-500 rounded text-sm text-white">
        Choose file
        <input
          type="file"
          accept=".json"
          className="hidden"
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) handleFile(file);
          }}
        />
      </label>
      {error && <p className="mt-4 text-red-400 text-sm">{error}</p>}
    </div>
  );
}
