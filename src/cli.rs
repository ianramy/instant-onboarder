//! Command-line interface definitions and parsing logic.
//!
//! This module utilizes the `clap` crate to define and parse the arguments, flags,
//! and options provided by the user at runtime. It serves as the primary external
//! control mechanism for the Instant Onboarder application, allowing users to alter
//! the default execution path (e.g., clearing the cache or forcing a setup prompt).

use clap::Parser;
use std::path::PathBuf;

/// Command-line argument structure for the Instant Onboarder application.
///
/// This struct defines the schema for the application's CLI. When the application
/// boots, `clap` automatically reads the raw `std::env::args`, maps them to these
/// fields, and handles the generation of the `--help` and `--version` output screens.
#[derive(Parser, Debug)]
#[command(
    author = "Instant Onboarder Team",
    version = "0.1.0",
    about = "AI-powered instant onboarding for codebases",
    long_about = "Instant Onboarder scans your codebase and generates comprehensive onboarding documentation using AI"
)]
pub struct Cli {
    /// The target directory path that the application should scan for source files.
    ///
    /// If the user does not explicitly provide a path when invoking the command,
    /// this defaults to `.` (the current working directory where the terminal is open).
    /// This path is heavily utilized by the `parser` module to walk the file tree.
    #[arg(default_value = ".")]
    pub target_dir: PathBuf,

    /// A flag to force the re-initialization of the user configuration.
    ///
    /// When passed (`--setup`), the application bypasses the standard execution flow.
    /// It actively deletes any existing configuration file on disk and drops the user
    /// directly into the interactive `dialoguer` prompts to input fresh API keys and
    /// select a new AI backend.
    #[arg(long, help = "Re-run the interactive setup to configure API keys")]
    pub setup: bool,

    /// A flag to wipe the local markdown cache directory.
    ///
    /// When passed (`--clear-cache`), the application safely destroys the existing
    /// cache directory where previously generated AI explanations are stored. This is
    /// useful when the codebase has been extensively updated or if the user switches
    /// to a smarter AI model and wants fresh explanations.
    #[arg(long, help = "Wipe all saved LLM responses from the cache")]
    pub clear_cache: bool,
}
