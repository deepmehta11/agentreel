use anyhow::Result;
use agentreel_core::Trajectory;
use std::path::PathBuf;

pub fn run(source: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let content = std::fs::read_to_string(&source)?;
    let trajectory = Trajectory::from_json(&content)?;

    let forked = trajectory.fork();
    let output_path = output.unwrap_or_else(|| {
        let stem = source.file_stem().unwrap().to_str().unwrap();
        source.with_file_name(format!("{}_fork.json", stem))
    });

    let json = forked.to_json()?;
    std::fs::write(&output_path, json)?;

    println!("Forked trajectory:");
    println!("  Parent: {}", trajectory.id);
    println!("  New:    {}", forked.id);
    println!("  Saved:  {}", output_path.display());

    Ok(())
}
