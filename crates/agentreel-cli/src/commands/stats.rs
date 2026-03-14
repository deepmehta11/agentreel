use anyhow::Result;
use agentreel_core::Trajectory;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(&path)?;
    let mut trajectory = Trajectory::from_json(&content)?;

    trajectory.compute_stats();

    let stats = trajectory.stats.as_ref().unwrap();

    println!("Trajectory Stats");
    println!("────────────────────────────────");
    println!("  Steps:      {}", stats.total_steps);
    println!("  LLM calls:  {}", stats.total_llm_calls);
    println!("  Tool calls: {}", stats.total_tool_calls);
    println!("  Errors:     {}", stats.errors_count);

    if let Some(ref tokens) = stats.total_tokens {
        let input = tokens.input_tokens.unwrap_or(0);
        let output = tokens.output_tokens.unwrap_or(0);
        println!("  Tokens:     {} in / {} out ({} total)", input, output, input + output);
    }

    if let Some(cost) = stats.total_cost_usd {
        println!("  Cost:       ${:.4}", cost);
    }

    if let Some(duration) = stats.total_duration_ms {
        if duration > 60_000.0 {
            println!("  Duration:   {:.1}m", duration / 60_000.0);
        } else if duration > 1_000.0 {
            println!("  Duration:   {:.1}s", duration / 1_000.0);
        } else {
            println!("  Duration:   {:.0}ms", duration);
        }
    }

    Ok(())
}
