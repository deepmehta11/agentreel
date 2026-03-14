use agentreel_core::config::Config;
use agentreel_core::Trajectory;
use agentreel_proxy::{ProxyConfig, ProxyServer, Provider, Recorder};
use anyhow::{bail, Result};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::info;

pub async fn run(
    cmd: Vec<String>,
    output: PathBuf,
    title: Option<String>,
    tags: Vec<String>,
) -> Result<()> {
    if cmd.is_empty() {
        bail!("No command provided. Usage: agentreel record -- python my_agent.py");
    }

    // Load config
    let config = Config::load().unwrap_or_default();

    // Merge tags from CLI and config
    let mut all_tags = tags;
    all_tags.extend(config.default_tags.clone());

    // Determine output path — use config trajectory_dir if default output
    let final_output = if output == PathBuf::from("trajectory.json") {
        // Default output — save to config's trajectory dir
        std::fs::create_dir_all(&config.trajectory_dir)?;
        let id = uuid::Uuid::new_v4();
        config.trajectory_dir.join(format!("{}.trajectory.json", id))
    } else {
        output
    };

    // Create trajectory
    let mut trajectory = Trajectory::new();
    trajectory.metadata.title = title;
    trajectory.metadata.tags = all_tags;
    trajectory.input = Some(agentreel_core::Input {
        prompt: Some(cmd.join(" ")),
        system_prompt: None,
        files: Vec::new(),
        context: None,
    });

    // Detect environment
    trajectory.metadata.environment = Some(agentreel_core::EnvironmentInfo {
        os: Some(std::env::consts::OS.to_string()),
        arch: Some(std::env::consts::ARCH.to_string()),
        runtime: None,
        extra: std::collections::HashMap::new(),
    });

    // Auto-detect provider from env vars
    let provider = Provider::from_env();
    let provider_name = match &provider {
        Provider::OpenAI => "OpenAI",
        Provider::Anthropic => "Anthropic",
        Provider::Auto => "Auto-detect",
    };

    // Start recorder and proxy
    let recorder = Recorder::new(trajectory);
    let recorder_clone = recorder.clone();

    let proxy_config = ProxyConfig {
        provider,
        openai_url: config.proxy.openai_upstream.clone()
            .or_else(|| std::env::var("OPENAI_BASE_URL").ok()),
        anthropic_url: config.proxy.anthropic_upstream.clone()
            .or_else(|| std::env::var("ANTHROPIC_BASE_URL").ok()),
    };

    let proxy = ProxyServer::new(proxy_config, recorder_clone);
    let (addr, proxy_handle) = proxy.start().await?;
    let proxy_url = format!("http://{}", addr);

    println!("AgentReel recording started");
    println!("  Provider: {}", provider_name);
    println!("  Proxy:    {}", proxy_url);
    println!("  Output:   {}", final_output.display());
    println!("  Command:  {}", cmd.join(" "));
    println!();

    // Spawn the child process with proxy env vars for all providers
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .env("OPENAI_BASE_URL", format!("{}/v1", proxy_url))
        .env("ANTHROPIC_BASE_URL", &proxy_url)
        .env("AGENTREEL_PROXY_URL", &proxy_url)
        .env("AGENTREEL_RECORDING", "1")
        .envs(std::env::vars().filter(|(k, _)| {
            k == "OPENAI_API_KEY"
                || k == "ANTHROPIC_API_KEY"
                || k == "GOOGLE_API_KEY"
                || k.starts_with("PATH")
                || k.starts_with("HOME")
                || k.starts_with("USER")
                || k.starts_with("LANG")
                || k.starts_with("TERM")
                || k.starts_with("SHELL")
                || k.starts_with("PYTHON")
                || k.starts_with("VIRTUAL_ENV")
                || k.starts_with("CONDA")
                || k.starts_with("NVM")
                || k.starts_with("NODE")
                || k.starts_with("CARGO")
                || k.starts_with("RUST")
        }))
        .spawn()?;

    let status = child.wait().await?;
    info!("Child process exited with: {}", status);

    // Stop proxy
    proxy_handle.abort();

    // Finalize trajectory
    let trajectory = recorder.finalize().await;
    let step_count = trajectory.steps.len();

    // Redact before saving if configured
    let json = trajectory.to_json()?;
    let final_json = if config.redact_by_default {
        agentreel_core::redact::redact(&json)
    } else {
        json
    };

    // Ensure output directory exists
    if let Some(parent) = final_output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&final_output, &final_json)?;

    println!();
    println!("Recording complete!");
    println!("  Steps recorded: {}", step_count);
    println!("  Saved to: {}", final_output.display());

    if !status.success() {
        println!("  Note: child process exited with {}", status);
    }

    Ok(())
}
