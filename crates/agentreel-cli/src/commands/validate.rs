use anyhow::Result;
use agentreel_core::Trajectory;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(&path)?;

    match Trajectory::from_json(&content) {
        Ok(t) => {
            println!("Valid trajectory (v{})", t.version);
            println!("  ID:    {}", t.id);
            println!("  Steps: {}", t.steps.len());
            if let Some(ref title) = t.metadata.title {
                println!("  Title: {}", title);
            }
        }
        Err(e) => {
            eprintln!("Invalid trajectory: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
