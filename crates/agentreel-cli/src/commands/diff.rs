use anyhow::Result;
use agentreel_core::Trajectory;
use std::path::PathBuf;

pub fn run(left: PathBuf, right: PathBuf) -> Result<()> {
    let left_content = std::fs::read_to_string(&left)?;
    let right_content = std::fs::read_to_string(&right)?;

    let left_traj = Trajectory::from_json(&left_content)?;
    let right_traj = Trajectory::from_json(&right_content)?;

    let diff = agentreel_core::diff::diff(&left_traj, &right_traj);
    println!("{}", diff);

    Ok(())
}
