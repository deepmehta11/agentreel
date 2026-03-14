"use client";

import { useState } from "react";
import type { Trajectory } from "@/lib/types";
import { TrajectoryViewer } from "@/components/TrajectoryViewer";
import { UploadZone } from "@/components/UploadZone";

export default function Home() {
  const [trajectory, setTrajectory] = useState<Trajectory | null>(null);

  if (trajectory) {
    return (
      <div>
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-sm text-gray-500">Viewing trajectory</h2>
          <button
            onClick={() => setTrajectory(null)}
            className="text-sm text-gray-400 hover:text-white"
          >
            Load another
          </button>
        </div>
        <TrajectoryViewer trajectory={trajectory} />
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto mt-12">
      <div className="text-center mb-8">
        <h1 className="text-4xl font-bold mb-3">AgentReel</h1>
        <p className="text-gray-400 text-lg">
          Browse, fork, and compare AI agent runs
        </p>
      </div>
      <UploadZone onLoad={setTrajectory} />

      <div className="mt-12 text-center">
        <p className="text-sm text-gray-600 mb-4">
          Or try the example trajectory:
        </p>
        <button
          onClick={async () => {
            try {
              const resp = await fetch("/example.trajectory.json");
              const data = await resp.json();
              setTrajectory(data);
            } catch {
              // Example file might not be available
            }
          }}
          className="text-sm text-brand-400 hover:text-brand-300"
        >
          Load example
        </button>
      </div>

      <div className="mt-16 grid grid-cols-3 gap-8 text-center text-sm">
        <div>
          <div className="text-2xl mb-2">{"\uD83C\uDFAC"}</div>
          <h3 className="font-medium mb-1">Record</h3>
          <p className="text-gray-500">
            Capture every LLM call, tool use, and decision your agent makes
          </p>
        </div>
        <div>
          <div className="text-2xl mb-2">{"\uD83D\uDD00"}</div>
          <h3 className="font-medium mb-1">Fork</h3>
          <p className="text-gray-500">
            Fork a run and re-run with a different model or prompt
          </p>
        </div>
        <div>
          <div className="text-2xl mb-2">{"\uD83D\uDD0D"}</div>
          <h3 className="font-medium mb-1">Compare</h3>
          <p className="text-gray-500">
            Diff two runs to see what changed across models or approaches
          </p>
        </div>
      </div>
    </div>
  );
}
