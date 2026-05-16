use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author = "Instant Onboarder Team",
    version = "0.1.0",
    about = "AI-powered instant onboarding for codebases",
    long_about = "Instant Onboarder scans your codebase and generates comprehensive onboarding documentation using AI"
)]
pub struct Cli {
    /// Target directory to scan (defaults to current directory)
    #[arg(default_value = ".")]
    pub target_dir: PathBuf,

    /// Force configuration re-initialization
    #[arg(long, help = "Re-run the interactive setup to configure API keys")]
    pub setup: bool,

    /// Clear cached LLM responses
    #[arg(long, help = "Wipe all saved LLM responses from the cache")]
    pub clear_cache: bool,
}

// Made with Bob
