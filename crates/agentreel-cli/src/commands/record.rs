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

    // Create trajectory
    let mut trajectory = Trajectory::new();
    trajectory.metadata.title = title;
    trajectory.metadata.tags = tags;
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

    let config = ProxyConfig {
        provider,
        openai_url: std::env::var("OPENAI_BASE_URL").ok(),
        anthropic_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
    };

    let proxy = ProxyServer::new(config, recorder_clone);
    let (addr, proxy_handle) = proxy.start().await?;
    let proxy_url = format!("http://{}", addr);

    println!("AgentReel recording started");
    println!("  Provider: {}", provider_name);
    println!("  Proxy:    {}", proxy_url);
    println!("  Command:  {}", cmd.join(" "));
    println!();

    // Spawn the child process with proxy env vars for all providers
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        // OpenAI SDK reads OPENAI_BASE_URL
        .env("OPENAI_BASE_URL", format!("{}/v1", proxy_url))
        // Anthropic SDK reads ANTHROPIC_BASE_URL
        .env("ANTHROPIC_BASE_URL", &proxy_url)
        // Generic marker for SDKs that check
        .env("AGENTREEL_PROXY_URL", &proxy_url)
        .env("AGENTREEL_RECORDING", "1")
        // Preserve existing API keys so auth works through the proxy
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
        }))
        .spawn()?;

    // Wait for child to finish
    let status = child.wait().await?;

    info!("Child process exited with: {}", status);

    // Stop proxy
    proxy_handle.abort();

    // Finalize trajectory
    let trajectory = recorder.finalize().await;
    let step_count = trajectory.steps.len();

    // Redact before saving
    let json = trajectory.to_json()?;
    let redacted = agentreel_core::redact::redact(&json);
    std::fs::write(&output, &redacted)?;

    println!();
    println!("Recording complete!");
    println!("  Steps recorded: {}", step_count);
    println!("  Saved to: {}", output.display());

    if !status.success() {
        println!("  Note: child process exited with {}", status);
    }

    Ok(())
}
