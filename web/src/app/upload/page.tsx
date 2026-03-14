"use client";

import { useState } from "react";
import type { Trajectory } from "@/lib/types";
import { UploadZone } from "@/components/UploadZone";
import { TrajectoryViewer } from "@/components/TrajectoryViewer";

export default function UploadPage() {
  const [trajectory, setTrajectory] = useState<Trajectory | null>(null);

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Upload Trajectory</h1>

      {trajectory ? (
        <div>
          <div className="flex justify-between items-center mb-6">
            <p className="text-sm text-gray-500">Preview</p>
            <div className="flex gap-4">
              <button
                onClick={() => {
                  const blob = new Blob(
                    [JSON.stringify(trajectory, null, 2)],
                    { type: "application/json" }
                  );
                  const url = URL.createObjectURL(blob);
                  const a = document.createElement("a");
                  a.href = url;
                  a.download = `${trajectory.id}.trajectory.json`;
                  a.click();
                  URL.revokeObjectURL(url);
                }}
                className="text-sm px-3 py-1 bg-gray-800 hover:bg-gray-700 rounded"
              >
                Download
              </button>
              <button
                onClick={() => setTrajectory(null)}
                className="text-sm text-gray-400 hover:text-white"
              >
                Upload another
              </button>
            </div>
          </div>
          <TrajectoryViewer trajectory={trajectory} />
        </div>
      ) : (
        <div className="max-w-2xl mx-auto mt-8">
          <UploadZone onLoad={setTrajectory} />
          <div className="mt-8 text-sm text-gray-600">
            <h3 className="font-medium text-gray-400 mb-2">
              Generate trajectory files with:
            </h3>
            <pre className="bg-gray-900 border border-gray-800 rounded p-4 text-gray-300">
              {`# Record a run
agentreel record -- python my_agent.py

# Or use the Python SDK
from agentreel import Recorder
recorder = Recorder(title="My run")
# ... record steps ...
trajectory = recorder.finalize()
trajectory.save("my_run.trajectory.json")`}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
