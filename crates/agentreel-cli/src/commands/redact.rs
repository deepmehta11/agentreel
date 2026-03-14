use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let content = std::fs::read_to_string(&path)?;

    let redacted = agentreel_core::redact::redact(&content);

    let output_path = output.unwrap_or(path);
    std::fs::write(&output_path, &redacted)?;

    println!("Redacted trajectory saved to: {}", output_path.display());

    Ok(())
}
