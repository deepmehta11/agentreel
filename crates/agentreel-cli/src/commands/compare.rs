use anyhow::Result;
use agentreel_core::Trajectory;
use std::path::PathBuf;

pub fn run(paths: Vec<PathBuf>, format: Option<String>) -> Result<()> {
    if paths.len() < 2 {
        anyhow::bail!("Need at least 2 trajectories to compare. Usage: agentreel compare a.json b.json [c.json ...]");
    }

    // Load all trajectories
    let mut trajectories: Vec<(String, Trajectory)> = Vec::new();
    for path in &paths {
        let content = std::fs::read_to_string(path)?;
        let mut traj = Trajectory::from_json(&content)?;
        traj.compute_stats();
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        trajectories.push((name, traj));
    }

    let fmt = format.as_deref().unwrap_or("text");

    match fmt {
        "json" => print_json_comparison(&trajectories)?,
        "markdown" | "md" => print_markdown_comparison(&trajectories)?,
        _ => print_text_comparison(&trajectories)?,
    }

    Ok(())
}

fn print_text_comparison(trajectories: &[(String, Trajectory)]) -> Result<()> {
    println!("Multi-Trajectory Comparison ({} runs)", trajectories.len());
    println!("{}", "=".repeat(80));
    println!();

    // Header row
    print!("{:<20}", "Metric");
    for (name, _) in trajectories {
        print!("{:<20}", name);
    }
    println!();
    print!("{:<20}", "");
    for _ in trajectories {
        print!("{:<20}", "---");
    }
    println!();

    // Model
    print!("{:<20}", "Model");
    for (_, t) in trajectories {
        let model = t.steps.iter()
            .find_map(|s| s.llm_call.as_ref().and_then(|c| c.model.as_deref()))
            .unwrap_or("?");
        print!("{:<20}", truncate(model, 18));
    }
    println!();

    // Steps
    print!("{:<20}", "Steps");
    for (_, t) in trajectories {
        print!("{:<20}", t.stats.as_ref().map(|s| s.total_steps).unwrap_or(0));
    }
    println!();

    // LLM Calls
    print!("{:<20}", "LLM Calls");
    for (_, t) in trajectories {
        print!("{:<20}", t.stats.as_ref().map(|s| s.total_llm_calls).unwrap_or(0));
    }
    println!();

    // Tool Calls
    print!("{:<20}", "Tool Calls");
    for (_, t) in trajectories {
        print!("{:<20}", t.stats.as_ref().map(|s| s.total_tool_calls).unwrap_or(0));
    }
    println!();

    // Tokens
    print!("{:<20}", "Tokens");
    for (_, t) in trajectories {
        let tokens = t.stats.as_ref()
            .and_then(|s| s.total_tokens.as_ref())
            .map(|t| t.input_tokens.unwrap_or(0) + t.output_tokens.unwrap_or(0))
            .unwrap_or(0);
        print!("{:<20}", format_tokens(tokens));
    }
    println!();

    // Cost
    print!("{:<20}", "Cost");
    for (_, t) in trajectories {
        let cost = t.stats.as_ref().and_then(|s| s.total_cost_usd).unwrap_or(0.0);
        print!("{:<20}", format!("${:.4}", cost));
    }
    println!();

    // Duration
    print!("{:<20}", "Duration");
    for (_, t) in trajectories {
        let dur = t.stats.as_ref().and_then(|s| s.total_duration_ms).unwrap_or(0.0);
        print!("{:<20}", format_duration(dur));
    }
    println!();

    // Errors
    print!("{:<20}", "Errors");
    for (_, t) in trajectories {
        print!("{:<20}", t.stats.as_ref().map(|s| s.errors_count).unwrap_or(0));
    }
    println!();

    // Outcome
    print!("{:<20}", "Outcome");
    for (_, t) in trajectories {
        let outcome = t.outcome.as_ref()
            .and_then(|o| o.status.as_ref())
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "?".to_string());
        print!("{:<20}", outcome);
    }
    println!();

    // Winner analysis
    println!();
    println!("Analysis:");

    // Cheapest
    if let Some((name, _)) = trajectories.iter()
        .filter(|(_, t)| t.stats.as_ref().and_then(|s| s.total_cost_usd).unwrap_or(0.0) > 0.0)
        .min_by(|(_, a), (_, b)| {
            let ca = a.stats.as_ref().and_then(|s| s.total_cost_usd).unwrap_or(f64::MAX);
            let cb = b.stats.as_ref().and_then(|s| s.total_cost_usd).unwrap_or(f64::MAX);
            ca.partial_cmp(&cb).unwrap()
        })
    {
        println!("  Cheapest:  {}", name);
    }

    // Fastest
    if let Some((name, _)) = trajectories.iter()
        .filter(|(_, t)| t.stats.as_ref().and_then(|s| s.total_duration_ms).unwrap_or(0.0) > 0.0)
        .min_by(|(_, a), (_, b)| {
            let da = a.stats.as_ref().and_then(|s| s.total_duration_ms).unwrap_or(f64::MAX);
            let db = b.stats.as_ref().and_then(|s| s.total_duration_ms).unwrap_or(f64::MAX);
            da.partial_cmp(&db).unwrap()
        })
    {
        println!("  Fastest:   {}", name);
    }

    // Fewest errors
    if let Some((name, _)) = trajectories.iter()
        .min_by_key(|(_, t)| t.stats.as_ref().map(|s| s.errors_count).unwrap_or(u32::MAX))
    {
        println!("  Fewest errors: {}", name);
    }

    Ok(())
}

fn print_json_comparison(trajectories: &[(String, Trajectory)]) -> Result<()> {
    let mut comparison = serde_json::Map::new();

    for (name, t) in trajectories {
        let stats = t.stats.as_ref();
        let tokens = stats
            .and_then(|s| s.total_tokens.as_ref())
            .map(|t| t.input_tokens.unwrap_or(0) + t.output_tokens.unwrap_or(0))
            .unwrap_or(0);

        comparison.insert(name.clone(), serde_json::json!({
            "id": t.id.to_string(),
            "model": t.steps.iter().find_map(|s| s.llm_call.as_ref().and_then(|c| c.model.as_deref())),
            "steps": stats.map(|s| s.total_steps).unwrap_or(0),
            "llm_calls": stats.map(|s| s.total_llm_calls).unwrap_or(0),
            "tool_calls": stats.map(|s| s.total_tool_calls).unwrap_or(0),
            "tokens": tokens,
            "cost_usd": stats.and_then(|s| s.total_cost_usd).unwrap_or(0.0),
            "duration_ms": stats.and_then(|s| s.total_duration_ms).unwrap_or(0.0),
            "errors": stats.map(|s| s.errors_count).unwrap_or(0),
            "outcome": t.outcome.as_ref().and_then(|o| o.status.as_ref()).map(|s| format!("{:?}", s)),
        }));
    }

    println!("{}", serde_json::to_string_pretty(&comparison)?);
    Ok(())
}

fn print_markdown_comparison(trajectories: &[(String, Trajectory)]) -> Result<()> {
    println!("# Trajectory Comparison\n");

    // Table header
    print!("| Metric |");
    for (name, _) in trajectories {
        print!(" {} |", name);
    }
    println!();
    print!("|--------|");
    for _ in trajectories {
        print!("--------|");
    }
    println!();

    // Rows
    let metrics = [
        ("Model", Box::new(|t: &Trajectory| {
            t.steps.iter()
                .find_map(|s| s.llm_call.as_ref().and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "?".to_string())
        }) as Box<dyn Fn(&Trajectory) -> String>),
        ("Steps", Box::new(|t: &Trajectory| {
            t.stats.as_ref().map(|s| s.total_steps.to_string()).unwrap_or_else(|| "0".to_string())
        })),
        ("Tokens", Box::new(|t: &Trajectory| {
            let tokens = t.stats.as_ref()
                .and_then(|s| s.total_tokens.as_ref())
                .map(|t| t.input_tokens.unwrap_or(0) + t.output_tokens.unwrap_or(0))
                .unwrap_or(0);
            format_tokens(tokens)
        })),
        ("Cost", Box::new(|t: &Trajectory| {
            format!("${:.4}", t.stats.as_ref().and_then(|s| s.total_cost_usd).unwrap_or(0.0))
        })),
        ("Duration", Box::new(|t: &Trajectory| {
            format_duration(t.stats.as_ref().and_then(|s| s.total_duration_ms).unwrap_or(0.0))
        })),
        ("Errors", Box::new(|t: &Trajectory| {
            t.stats.as_ref().map(|s| s.errors_count.to_string()).unwrap_or_else(|| "0".to_string())
        })),
    ];

    for (metric, extractor) in &metrics {
        print!("| {} |", metric);
        for (_, t) in trajectories {
            print!(" {} |", extractor(t));
        }
        println!();
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max - 3]) }
}

fn format_tokens(t: u64) -> String {
    if t > 1_000_000 { format!("{:.1}M", t as f64 / 1_000_000.0) }
    else if t > 1_000 { format!("{:.1}k", t as f64 / 1_000.0) }
    else { t.to_string() }
}

fn format_duration(ms: f64) -> String {
    if ms > 60_000.0 { format!("{:.1}m", ms / 60_000.0) }
    else if ms > 1_000.0 { format!("{:.1}s", ms / 1_000.0) }
    else { format!("{:.0}ms", ms) }
}
