"use client";

import { useState, useCallback } from "react";
import type { Trajectory } from "@/lib/types";
import { TrajectoryHeader } from "./TrajectoryHeader";
import { StepTimeline } from "./StepTimeline";
import { VisualTimeline } from "./VisualTimeline";
import { SearchBar } from "./SearchBar";
import { ReplayDebugger } from "./ReplayDebugger";
import { CostWaterfall } from "./CostWaterfall";
import { SmartAnalysis } from "./SmartAnalysis";
import { ExportButton } from "./ExportButton";

type Tab = "steps" | "cost" | "analysis";

export function TrajectoryViewer({ trajectory }: { trajectory: Trajectory }) {
  const [matchedIndexes, setMatchedIndexes] = useState<Set<number> | null>(null);
  const [expandedIndex, setExpandedIndex] = useState<number | null>(null);
  const [showReplay, setShowReplay] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("steps");

  const handleSearchResults = useCallback((results: Set<number> | null) => {
    setMatchedIndexes(results);
  }, []);

  const handleSelectStep = useCallback((index: number) => {
    setExpandedIndex(index);
    setActiveTab("steps");
    setTimeout(() => setExpandedIndex(null), 100);
  }, []);

  return (
    <div className="space-y-6">
      {showReplay && (
        <ReplayDebugger steps={trajectory.steps} onClose={() => setShowReplay(false)} />
      )}

      <TrajectoryHeader trajectory={trajectory} />

      {/* Action bar */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => setShowReplay(true)}
          className="text-xs px-3 py-1.5 bg-brand-600 hover:bg-brand-500 rounded text-white font-medium"
        >
          Replay
        </button>
        <ExportButton trajectory={trajectory} />
        <button
          onClick={() => {
            const blob = new Blob([JSON.stringify(trajectory, null, 2)], { type: "application/json" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = `${trajectory.id}.trajectory.json`;
            a.click();
            URL.revokeObjectURL(url);
          }}
          className="text-xs px-3 py-1.5 bg-gray-800 hover:bg-gray-700 rounded border border-gray-700 text-gray-300"
        >
          Download JSON
        </button>
      </div>

      {/* Visual Timeline */}
      {trajectory.steps.length > 0 && (
        <VisualTimeline steps={trajectory.steps} onSelectStep={handleSelectStep} />
      )}

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-gray-800">
        {(["steps", "cost", "analysis"] as Tab[]).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 text-sm capitalize border-b-2 transition-colors ${
              activeTab === tab
                ? "border-brand-500 text-white"
                : "border-transparent text-gray-500 hover:text-gray-300"
            }`}
          >
            {tab === "cost" ? "Cost Breakdown" : tab === "analysis" ? "Smart Analysis" : tab}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {activeTab === "steps" && (
        <div className="space-y-4">
          <SearchBar steps={trajectory.steps} onResults={handleSearchResults} />
          <div>
            {matchedIndexes != null && (
              <div className="text-sm text-gray-500 mb-2">
                {matchedIndexes.size} of {trajectory.steps.length} steps shown
              </div>
            )}
            <StepTimeline
              steps={trajectory.steps}
              matchedIndexes={matchedIndexes}
              expandedIndex={expandedIndex}
            />
          </div>
        </div>
      )}

      {activeTab === "cost" && <CostWaterfall steps={trajectory.steps} />}
      {activeTab === "analysis" && <SmartAnalysis trajectory={trajectory} />}
    </div>
  );
}
