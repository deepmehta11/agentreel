use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(name = "agentreel", version, about = "Record, replay, and share AI agent runs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Record an agent run by proxying LLM API calls
    Record {
        /// Command to run (e.g., "python my_agent.py")
        #[arg(last = true)]
        cmd: Vec<String>,

        /// Output file for the trajectory
        #[arg(short, long, default_value = "trajectory.json")]
        output: PathBuf,

        /// Title for this run
        #[arg(short, long)]
        title: Option<String>,

        /// Tags for this run (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },

    /// View a trajectory file
    View {
        /// Path to the trajectory file
        path: PathBuf,

        /// Show full details (including message content)
        #[arg(long)]
        full: bool,
    },

    /// Show stats for a trajectory
    Stats {
        /// Path to the trajectory file
        path: PathBuf,
    },

    /// Compare two trajectory files
    Diff {
        /// Left trajectory file
        left: PathBuf,

        /// Right trajectory file
        right: PathBuf,
    },

    /// Fork a trajectory for re-running with different parameters
    Fork {
        /// Path to the source trajectory
        source: PathBuf,

        /// Output path for the forked trajectory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate a trajectory file against the schema
    Validate {
        /// Path to the trajectory file
        path: PathBuf,
    },

    /// Redact secrets from a trajectory file
    Redact {
        /// Path to the trajectory file
        path: PathBuf,

        /// Write redacted output to a new file (default: overwrite in place)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List local trajectories
    List {
        /// Directory to scan (default: ~/.agentreel/trajectories/)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Filter by tags
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Max number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },

    /// Compare multiple trajectories side by side
    Compare {
        /// Trajectory files to compare
        paths: Vec<PathBuf>,

        /// Output format (text, json, markdown)
        #[arg(long, default_value = "text")]
        format: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agentreel=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Record { cmd, output, title, tags } => {
            commands::record::run(cmd, output, title, tags).await
        }
        Commands::View { path, full } => {
            commands::view::run(path, full)
        }
        Commands::Stats { path } => {
            commands::stats::run(path)
        }
        Commands::Diff { left, right } => {
            commands::diff::run(left, right)
        }
        Commands::Fork { source, output } => {
            commands::fork::run(source, output)
        }
        Commands::Validate { path } => {
            commands::validate::run(path)
        }
        Commands::Redact { path, output } => {
            commands::redact::run(path, output)
        }
        Commands::List { dir, tags, limit } => {
            commands::list::run(dir, tags, limit)
        }
        Commands::Compare { paths, format } => {
            commands::compare::run(paths, format)
        }
    }
}
