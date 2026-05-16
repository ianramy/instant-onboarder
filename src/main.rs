mod cli;
mod config;
mod engine;
mod errors;
mod parser;
mod ui;

use clap::Parser;
use cli::Cli;
use config::AppConfig;
use engine::{AiClient, CacheManager};
use miette::{Context, IntoDiagnostic};
use std::fs;

#[tokio::main]
async fn main() -> miette::Result<()> {
    // Parse command-line arguments
    let args = Cli::parse();

    // Handle --clear-cache flag
    if args.clear_cache {
        let config = AppConfig::load_or_init()
            .into_diagnostic()
            .wrap_err("Failed to load configuration")?;

        let cache_manager = CacheManager::new(config.cache_dir.clone())
            .into_diagnostic()
            .wrap_err("Failed to initialize cache manager")?;

        cache_manager
            .clear_cache()
            .into_diagnostic()
            .wrap_err("Failed to clear cache")?;

        println!("✓ Cache cleared successfully");
        return Ok(());
    }

    // Handle --setup flag (force config re-initialization)
    if args.setup {
        // Delete existing config to force re-initialization
        if let Ok(config_path) = AppConfig::config_path() {
            if config_path.exists() {
                fs::remove_file(&config_path)
                    .into_diagnostic()
                    .wrap_err("Failed to remove existing config")?;
            }
        }

        let _config = AppConfig::load_or_init()
            .into_diagnostic()
            .wrap_err("Failed to initialize configuration")?;

        println!("\n✓ Configuration saved successfully!");
        return Ok(());
    }

    // Standard Execution Flow

    // Load configuration
    let config = AppConfig::load_or_init()
        .into_diagnostic()
        .wrap_err("Failed to load configuration")?;

    // Validate configuration
    config
        .validate()
        .into_diagnostic()
        .wrap_err("Configuration validation failed")?;

    // Initialize CacheManager
    let cache_manager = CacheManager::new(config.cache_dir.clone())
        .into_diagnostic()
        .wrap_err("Failed to initialize cache manager")?;

    // Initialize AiClient
    let ai_client = AiClient::new(config.clone());

    // Scan the target directory
    println!("🔍 Scanning directory: {}", args.target_dir.display());

    let files = parser::scan_directory(&args.target_dir)
        .into_diagnostic()
        .wrap_err("Failed to scan directory")?;

    if files.is_empty() {
        println!("\n⚠ No valid source files found in the target directory.");
        println!("  Make sure you're pointing to a directory with source code files.");
        return Ok(());
    }

    // Print scan summary
    println!("✓ Found {} files", files.len());

    let cache_count = cache_manager
        .cache_count()
        .into_diagnostic()
        .wrap_err("Failed to get cache count")?;

    if cache_count > 0 {
        println!("📦 {} files already cached", cache_count);
    }

    println!("\n🚀 Launching interactive TUI...\n");

    // Launch the TUI
    ui::run_tui(files, ai_client, cache_manager)
        .await
        .into_diagnostic()
        .wrap_err("TUI execution failed")?;

    println!("\n👋 Thanks for using Instant Onboarder!");

    Ok(())
}

// Made with Bob
