use anyhow::Result;
use std::path::PathBuf;

pub fn run(dir: Option<PathBuf>, tags: Vec<String>, limit: Option<usize>) -> Result<()> {
    let traj_dir = dir.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".agentreel")
            .join("trajectories")
    });

    if !traj_dir.exists() {
        println!("No trajectories found. Directory does not exist: {}", traj_dir.display());
        println!("Record a run with: agentreel record -- <command>");
        return Ok(());
    }

    let mut entries: Vec<TrajectoryEntry> = Vec::new();

    for entry in std::fs::read_dir(&traj_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Parse just the top-level fields for performance
        let raw: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = raw.get("id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let title = raw
            .get("metadata")
            .and_then(|m| m.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();
        let created = raw
            .get("metadata")
            .and_then(|m| m.get("created_at"))
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let entry_tags: Vec<String> = raw
            .get("metadata")
            .and_then(|m| m.get("tags"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let steps = raw
            .get("stats")
            .and_then(|s| s.get("total_steps"))
            .and_then(|v| v.as_u64())
            .or_else(|| raw.get("steps").and_then(|v| v.as_array()).map(|a| a.len() as u64))
            .unwrap_or(0);
        let cost = raw
            .get("stats")
            .and_then(|s| s.get("total_cost_usd"))
            .and_then(|v| v.as_f64());
        let duration = raw
            .get("stats")
            .and_then(|s| s.get("total_duration_ms"))
            .and_then(|v| v.as_f64());
        let model = raw
            .get("metadata")
            .and_then(|m| m.get("model"))
            .and_then(|m| m.get("model_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("").to_string();
        let outcome = raw
            .get("outcome")
            .and_then(|o| o.get("status"))
            .and_then(|v| v.as_str())
            .or_else(|| raw.get("summary").and_then(|s| s.get("outcome")).and_then(|v| v.as_str()))
            .unwrap_or("?")
            .to_string();

        // Filter by tags if specified
        if !tags.is_empty() && !tags.iter().any(|t| entry_tags.contains(t)) {
            continue;
        }

        entries.push(TrajectoryEntry {
            id,
            title,
            created,
            steps,
            cost,
            duration,
            model,
            outcome,
            tags: entry_tags,
            path,
        });
    }

    // Sort by creation date (newest first)
    entries.sort_by(|a, b| b.created.cmp(&a.created));

    // Apply limit
    if let Some(limit) = limit {
        entries.truncate(limit);
    }

    if entries.is_empty() {
        println!("No trajectories found in {}", traj_dir.display());
        return Ok(());
    }

    // Print table
    println!(
        "{:<8} {:<30} {:<6} {:<10} {:<10} {:<8} {}",
        "STATUS", "TITLE", "STEPS", "COST", "DURATION", "MODEL", "CREATED"
    );
    println!("{}", "-".repeat(100));

    for e in &entries {
        let status_icon = match e.outcome.as_str() {
            "success" => "✅",
            "failure" => "❌",
            "partial" => "⚠️",
            _ => "  ",
        };
        let cost_str = e.cost.map(|c| format!("${:.4}", c)).unwrap_or_else(|| "-".to_string());
        let dur_str = e.duration.map(|d| {
            if d > 60_000.0 { format!("{:.1}m", d / 60_000.0) }
            else if d > 1_000.0 { format!("{:.1}s", d / 1_000.0) }
            else { format!("{:.0}ms", d) }
        }).unwrap_or_else(|| "-".to_string());
        let model_short = if e.model.len() > 8 {
            e.model.chars().take(8).collect::<String>()
        } else {
            e.model.clone()
        };
        let created_short = e.created.get(..10).unwrap_or(&e.created);
        let title_short: String = e.title.chars().take(28).collect();

        println!(
            "{}  {:<30} {:<6} {:<10} {:<10} {:<8} {}",
            status_icon, title_short, e.steps, cost_str, dur_str, model_short, created_short
        );
    }

    println!("\n{} trajectories in {}", entries.len(), traj_dir.display());

    Ok(())
}

struct TrajectoryEntry {
    id: String,
    title: String,
    created: String,
    steps: u64,
    cost: Option<f64>,
    duration: Option<f64>,
    model: String,
    outcome: String,
    tags: Vec<String>,
    path: PathBuf,
}
