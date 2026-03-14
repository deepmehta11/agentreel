use crate::trajectory::{StepType, Trajectory};
use std::fmt;

/// A comparison between two trajectories.
#[derive(Debug)]
pub struct TrajectoryDiff {
    pub left_id: String,
    pub right_id: String,
    pub metadata_diffs: Vec<FieldDiff>,
    pub step_diffs: Vec<StepDiff>,
    pub stats_comparison: Option<StatsComparison>,
}

#[derive(Debug)]
pub struct FieldDiff {
    pub field: String,
    pub left: String,
    pub right: String,
}

#[derive(Debug)]
pub enum StepDiff {
    /// Step exists only in left trajectory.
    LeftOnly { index: u32, summary: String },
    /// Step exists only in right trajectory.
    RightOnly { index: u32, summary: String },
    /// Step exists in both but differs.
    Changed {
        left_index: u32,
        right_index: u32,
        changes: Vec<String>,
    },
    /// Steps are identical.
    Same { left_index: u32, right_index: u32 },
}

#[derive(Debug)]
pub struct StatsComparison {
    pub left_steps: u32,
    pub right_steps: u32,
    pub left_llm_calls: u32,
    pub right_llm_calls: u32,
    pub left_tool_calls: u32,
    pub right_tool_calls: u32,
    pub left_tokens: u64,
    pub right_tokens: u64,
    pub left_cost: f64,
    pub right_cost: f64,
    pub left_duration_ms: f64,
    pub right_duration_ms: f64,
    pub left_errors: u32,
    pub right_errors: u32,
}

/// Compare two trajectories and return a diff.
pub fn diff(left: &Trajectory, right: &Trajectory) -> TrajectoryDiff {
    let mut metadata_diffs = Vec::new();

    // Compare metadata
    if left.metadata.title != right.metadata.title {
        metadata_diffs.push(FieldDiff {
            field: "title".to_string(),
            left: left.metadata.title.clone().unwrap_or_default(),
            right: right.metadata.title.clone().unwrap_or_default(),
        });
    }

    let left_model = left.metadata.model.as_ref().and_then(|m| m.model_id.as_deref());
    let right_model = right.metadata.model.as_ref().and_then(|m| m.model_id.as_deref());
    if left_model != right_model {
        metadata_diffs.push(FieldDiff {
            field: "model".to_string(),
            left: left_model.unwrap_or("unknown").to_string(),
            right: right_model.unwrap_or("unknown").to_string(),
        });
    }

    let left_agent = left.metadata.agent.as_ref().and_then(|a| a.name.as_deref());
    let right_agent = right.metadata.agent.as_ref().and_then(|a| a.name.as_deref());
    if left_agent != right_agent {
        metadata_diffs.push(FieldDiff {
            field: "agent".to_string(),
            left: left_agent.unwrap_or("unknown").to_string(),
            right: right_agent.unwrap_or("unknown").to_string(),
        });
    }

    // Compare steps using LCS-like alignment
    let step_diffs = diff_steps(left, right);

    // Compare stats
    let stats_comparison = diff_stats(left, right);

    TrajectoryDiff {
        left_id: left.id.to_string(),
        right_id: right.id.to_string(),
        metadata_diffs,
        step_diffs,
        stats_comparison: Some(stats_comparison),
    }
}

fn diff_steps(left: &Trajectory, right: &Trajectory) -> Vec<StepDiff> {
    let mut diffs = Vec::new();
    let max_len = left.steps.len().max(right.steps.len());

    for i in 0..max_len {
        match (left.steps.get(i), right.steps.get(i)) {
            (Some(l), Some(r)) => {
                let mut changes = Vec::new();

                if l.step_type != r.step_type {
                    changes.push(format!(
                        "type: {:?} -> {:?}",
                        l.step_type, r.step_type
                    ));
                }

                // Compare LLM calls
                if l.step_type == StepType::LlmCall && r.step_type == StepType::LlmCall {
                    let l_model = l.llm_call.as_ref().and_then(|c| c.model.as_deref());
                    let r_model = r.llm_call.as_ref().and_then(|c| c.model.as_deref());
                    if l_model != r_model {
                        changes.push(format!(
                            "model: {} -> {}",
                            l_model.unwrap_or("?"),
                            r_model.unwrap_or("?")
                        ));
                    }

                    let l_stop = l.llm_call.as_ref().and_then(|c| c.stop_reason.as_ref());
                    let r_stop = r.llm_call.as_ref().and_then(|c| c.stop_reason.as_ref());
                    if l_stop != r_stop {
                        changes.push(format!("stop_reason: {:?} -> {:?}", l_stop, r_stop));
                    }
                }

                // Compare tool calls
                if l.step_type == StepType::ToolCall && r.step_type == StepType::ToolCall {
                    let l_name = l.tool_call.as_ref().map(|c| c.name.as_str());
                    let r_name = r.tool_call.as_ref().map(|c| c.name.as_str());
                    if l_name != r_name {
                        changes.push(format!(
                            "tool: {} -> {}",
                            l_name.unwrap_or("?"),
                            r_name.unwrap_or("?")
                        ));
                    }

                    let l_status = l.tool_call.as_ref().and_then(|c| c.status.as_ref());
                    let r_status = r.tool_call.as_ref().and_then(|c| c.status.as_ref());
                    if l_status != r_status {
                        changes.push(format!("status: {:?} -> {:?}", l_status, r_status));
                    }
                }

                // Compare duration
                if l.duration_ms != r.duration_ms {
                    changes.push(format!(
                        "duration: {:.0}ms -> {:.0}ms",
                        l.duration_ms.unwrap_or(0.0),
                        r.duration_ms.unwrap_or(0.0)
                    ));
                }

                if changes.is_empty() {
                    diffs.push(StepDiff::Same {
                        left_index: l.index,
                        right_index: r.index,
                    });
                } else {
                    diffs.push(StepDiff::Changed {
                        left_index: l.index,
                        right_index: r.index,
                        changes,
                    });
                }
            }
            (Some(l), None) => {
                diffs.push(StepDiff::LeftOnly {
                    index: l.index,
                    summary: step_summary(l),
                });
            }
            (None, Some(r)) => {
                diffs.push(StepDiff::RightOnly {
                    index: r.index,
                    summary: step_summary(r),
                });
            }
            (None, None) => break,
        }
    }

    diffs
}

fn step_summary(step: &crate::trajectory::Step) -> String {
    match step.step_type {
        StepType::LlmCall => {
            let model = step
                .llm_call
                .as_ref()
                .and_then(|c| c.model.as_deref())
                .unwrap_or("?");
            format!("llm_call({})", model)
        }
        StepType::ToolCall => {
            let name = step
                .tool_call
                .as_ref()
                .map(|c| c.name.as_str())
                .unwrap_or("?");
            format!("tool_call({})", name)
        }
        StepType::ToolResult => {
            let name = step
                .tool_result
                .as_ref()
                .map(|r| r.tool_name.as_str())
                .unwrap_or("?");
            format!("tool_result({})", name)
        }
        StepType::HumanInput => "human_input".to_string(),
        StepType::Error => {
            let msg = step
                .error
                .as_ref()
                .and_then(|e| e.message.as_deref())
                .unwrap_or("?");
            format!("error({})", msg)
        }
        StepType::Thought => {
            let content = step
                .thought
                .as_ref()
                .map(|t| &t.content[..t.content.len().min(50)])
                .unwrap_or("?");
            format!("thought({})", content)
        }
        StepType::AgentDecision => {
            let decision = step
                .agent_decision
                .as_ref()
                .map(|d| &d.decision[..d.decision.len().min(50)])
                .unwrap_or("?");
            format!("decision({})", decision)
        }
        StepType::FileOperation => {
            let op = step
                .file_operation
                .as_ref()
                .map(|f| format!("{:?}({})", f.operation, f.path))
                .unwrap_or_else(|| "?".to_string());
            format!("file({})", op)
        }
        StepType::Retry => "retry".to_string(),
        StepType::Screenshot => "screenshot".to_string(),
        StepType::NetworkRequest => "network_request".to_string(),
        StepType::Handoff => "handoff".to_string(),
        StepType::Checkpoint => "checkpoint".to_string(),
    }
}

fn diff_stats(left: &Trajectory, right: &Trajectory) -> StatsComparison {
    let l = left.stats.as_ref();
    let r = right.stats.as_ref();

    let l_tokens = l
        .and_then(|s| s.total_tokens.as_ref())
        .map(|t| t.input_tokens.unwrap_or(0) + t.output_tokens.unwrap_or(0))
        .unwrap_or(0);
    let r_tokens = r
        .and_then(|s| s.total_tokens.as_ref())
        .map(|t| t.input_tokens.unwrap_or(0) + t.output_tokens.unwrap_or(0))
        .unwrap_or(0);

    StatsComparison {
        left_steps: l.map(|s| s.total_steps).unwrap_or(left.steps.len() as u32),
        right_steps: r.map(|s| s.total_steps).unwrap_or(right.steps.len() as u32),
        left_llm_calls: l.map(|s| s.total_llm_calls).unwrap_or(0),
        right_llm_calls: r.map(|s| s.total_llm_calls).unwrap_or(0),
        left_tool_calls: l.map(|s| s.total_tool_calls).unwrap_or(0),
        right_tool_calls: r.map(|s| s.total_tool_calls).unwrap_or(0),
        left_tokens: l_tokens,
        right_tokens: r_tokens,
        left_cost: l.and_then(|s| s.total_cost_usd).unwrap_or(0.0),
        right_cost: r.and_then(|s| s.total_cost_usd).unwrap_or(0.0),
        left_duration_ms: l.and_then(|s| s.total_duration_ms).unwrap_or(0.0),
        right_duration_ms: r.and_then(|s| s.total_duration_ms).unwrap_or(0.0),
        left_errors: l.map(|s| s.errors_count).unwrap_or(0),
        right_errors: r.map(|s| s.errors_count).unwrap_or(0),
    }
}

impl fmt::Display for TrajectoryDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Trajectory Diff")?;
        writeln!(f, "  Left:  {}", self.left_id)?;
        writeln!(f, "  Right: {}", self.right_id)?;
        writeln!(f)?;

        if !self.metadata_diffs.is_empty() {
            writeln!(f, "Metadata Changes:")?;
            for d in &self.metadata_diffs {
                writeln!(f, "  {}: {} -> {}", d.field, d.left, d.right)?;
            }
            writeln!(f)?;
        }

        writeln!(f, "Steps:")?;
        for d in &self.step_diffs {
            match d {
                StepDiff::Same { left_index, .. } => {
                    writeln!(f, "  = Step {} (identical)", left_index)?;
                }
                StepDiff::Changed {
                    left_index,
                    changes,
                    ..
                } => {
                    writeln!(f, "  ~ Step {} (changed)", left_index)?;
                    for c in changes {
                        writeln!(f, "      {}", c)?;
                    }
                }
                StepDiff::LeftOnly { index, summary } => {
                    writeln!(f, "  - Step {} (left only): {}", index, summary)?;
                }
                StepDiff::RightOnly { index, summary } => {
                    writeln!(f, "  + Step {} (right only): {}", index, summary)?;
                }
            }
        }

        if let Some(ref stats) = self.stats_comparison {
            writeln!(f)?;
            writeln!(f, "Stats Comparison:")?;
            writeln!(
                f,
                "  Steps:      {} vs {}",
                stats.left_steps, stats.right_steps
            )?;
            writeln!(
                f,
                "  LLM calls:  {} vs {}",
                stats.left_llm_calls, stats.right_llm_calls
            )?;
            writeln!(
                f,
                "  Tool calls: {} vs {}",
                stats.left_tool_calls, stats.right_tool_calls
            )?;
            writeln!(
                f,
                "  Tokens:     {} vs {}",
                stats.left_tokens, stats.right_tokens
            )?;
            writeln!(
                f,
                "  Cost:       ${:.4} vs ${:.4}",
                stats.left_cost, stats.right_cost
            )?;
            writeln!(
                f,
                "  Duration:   {:.0}ms vs {:.0}ms",
                stats.left_duration_ms, stats.right_duration_ms
            )?;
            writeln!(
                f,
                "  Errors:     {} vs {}",
                stats.left_errors, stats.right_errors
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::*;
    use chrono::Utc;

    fn make_trajectory(steps: Vec<Step>) -> Trajectory {
        let mut t = Trajectory::new();
        t.steps = steps;
        t.compute_stats();
        t
    }

    #[test]
    fn test_diff_identical() {
        let mut step = Step::new(0, StepType::LlmCall);
        step.duration_ms = Some(500.0);
        step.llm_call = Some(LlmCall {
            model: Some("gpt-4o".to_string()),
            messages: Vec::new(),
            response: None,
            stop_reason: None,
            provider: None,
            system_prompt: None,
            response_blocks: Vec::new(),
            config: None,
            available_tools: Vec::new(),
            http: None,
            thinking: None,
        });

        let left = make_trajectory(vec![step.clone()]);
        let right = make_trajectory(vec![step]);

        let d = diff(&left, &right);
        assert!(matches!(&d.step_diffs[0], StepDiff::Same { .. }));
    }

    #[test]
    fn test_diff_different_models() {
        let mut left = Trajectory::new();
        left.metadata.model = Some(ModelInfo {
            provider: Some("openai".to_string()),
            model_id: Some("gpt-4o".to_string()),
            parameters: None,
        });

        let mut right = Trajectory::new();
        right.metadata.model = Some(ModelInfo {
            provider: Some("anthropic".to_string()),
            model_id: Some("claude-opus-4-6".to_string()),
            parameters: None,
        });

        let d = diff(&left, &right);
        assert!(d.metadata_diffs.iter().any(|d| d.field == "model"));
    }

    #[test]
    fn test_diff_extra_steps() {
        let mut step = Step::new(0, StepType::LlmCall);
        step.duration_ms = Some(500.0);

        let left = make_trajectory(vec![step.clone()]);
        let mut step2 = step.clone();
        step2.index = 1;
        let right = make_trajectory(vec![step, step2]);

        let d = diff(&left, &right);
        assert_eq!(d.step_diffs.len(), 2);
        assert!(matches!(&d.step_diffs[1], StepDiff::RightOnly { .. }));
    }

    #[test]
    fn test_diff_display() {
        let left = make_trajectory(vec![]);
        let right = make_trajectory(vec![]);
        let d = diff(&left, &right);
        let output = format!("{}", d);
        assert!(output.contains("Trajectory Diff"));
    }
}
