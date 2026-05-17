//! Primary application entry point and execution bootloader.
//!
//! The `main` module is responsible for orchestrating the lifecycle of the application.
//! It handles the initial reading of command-line arguments, delegates configuration checks,
//! instantiates the core engine services, initiates the file system parsing phase, and
//! ultimately hands off execution to the asynchronous TUI run loop.

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

/// Main asynchronous execution function.
///
/// This function acts as the global coordinator. It heavily utilizes `miette`'s
/// `into_diagnostic()` trait to convert internal application errors into rich, readable
/// terminal outputs. It intercepts utility flags (like `--clear-cache` and `--setup`)
/// to run short-circuit administrative tasks. If no administrative flags are present,
/// it parses the target directory, pre-warms the cache manager and AI clients,
/// and launches the TUI rendering process.
#[tokio::main]
async fn main() -> miette::Result<()> {
    // 1. Ingest command-line arguments provided by the user.
    let args = Cli::parse();

    // 2. Handle the administrative --clear-cache flag and terminate safely.
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

    // 3. Handle the administrative --setup flag to force a configuration rebuild.
    if args.setup {
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

    // 4. Standard Application Flow: Load configuration or prompt user for initial setup.
    let config = AppConfig::load_or_init()
        .into_diagnostic()
        .wrap_err("Failed to load configuration")?;

    config
        .validate()
        .into_diagnostic()
        .wrap_err("Configuration validation failed")?;

    // 5. Initialize the core infrastructure (Cache and AI Client).
    let cache_manager = CacheManager::new(config.cache_dir.clone())
        .into_diagnostic()
        .wrap_err("Failed to initialize cache manager")?;

    let ai_client = AiClient::new(config.clone());

    // 6. Execute the file discovery phase on the target directory.
    println!("Scanning directory: {}", args.target_dir.display());

    let files = parser::scan_directory(&args.target_dir)
        .into_diagnostic()
        .wrap_err("Failed to scan directory")?;

    if files.is_empty() {
        println!("\nNo valid source files found in the target directory.");
        println!("  Make sure you're pointing to a directory with source code files.");
        return Ok(());
    }

    println!("✓ Found {} files", files.len());

    let cache_count = cache_manager
        .cache_count()
        .into_diagnostic()
        .wrap_err("Failed to get cache count")?;

    if cache_count > 0 {
        println!("{} files already cached", cache_count);
    }

    println!("\nLaunching interactive TUI...\n");

    // 7. Surrender control flow to the asynchronous TUI rendering loop.
    ui::run_tui(files, ai_client, cache_manager)
        .await
        .into_diagnostic()
        .wrap_err("TUI execution failed")?;

    println!("\nThanks for using Instant Onboarder!");

    Ok(())
}
