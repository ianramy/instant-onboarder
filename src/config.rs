//! Configuration management, persistence, and interactive user setup.
//!
//! This module is responsible for loading, validating, and saving the user's preferences
//! and API keys. It leverages the `directories` crate to safely locate the correct
//! standardized config directories across Windows, macOS, and Linux, ensuring the
//! application doesn't clutter the user's home directory unnecessarily.

use crate::errors::OnboarderError;
use dialoguer::{Input, Select};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration state containing AI preferences and access credentials.
///
/// This struct acts as the central source of truth for the application's environment.
/// It dictates whether the engine should communicate with external APIs (like IBM Watsonx),
/// local daemons (like Ollama), or bypass AI entirely. It implements `Serialize` and
/// `Deserialize` to easily convert to and from a local `config.json` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// The user's optional API key for authenticating with IBM Cloud services.
    pub watsonx_api_key: Option<String>,
    /// The specific Project ID associated with the user's IBM watsonx.ai environment.
    pub watsonx_project_id: Option<String>,
    /// Boolean flag determining if requests should route to a local `localhost:11434` instance.
    pub use_local_ollama: bool,
    /// Boolean flag bypassing all AI models in favor of the lightning-fast static AST parser.
    pub use_local_outline: bool,
    /// The dynamically calculated system path where generated markdown files are cached.
    pub cache_dir: PathBuf,
}

impl AppConfig {
    /// Dynamically resolves the path to the application's primary configuration file.
    ///
    /// This function utilizes the OS-specific standard directories (e.g., `~/.config/instant-onboarder/`
    /// on Linux, or `AppData\Roaming\...` on Windows). It also proactively creates the
    /// directory tree if it does not yet exist to prevent subsequent write errors.
    pub fn config_path() -> Result<PathBuf, OnboarderError> {
        let proj_dirs = ProjectDirs::from("com", "instant-onboarder", "instant-onboarder")
            .ok_or_else(|| {
                OnboarderError::ConfigError(
                    "Could not determine config directory for your system".to_string(),
                )
            })?;

        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir)?;

        Ok(config_dir.join("config.json"))
    }

    /// Dynamically resolves the path to the application's persistent cache directory.
    ///
    /// Similar to `config_path`, this uses OS-specific caching standards (e.g., `~/.cache/...`
    /// on Linux) to store heavy string data. Using the designated cache directory ensures
    /// that these generated files don't interfere with the user's cloud backups or configurations.
    fn default_cache_dir() -> Result<PathBuf, OnboarderError> {
        let proj_dirs = ProjectDirs::from("com", "instant-onboarder", "instant-onboarder")
            .ok_or_else(|| {
                OnboarderError::ConfigError(
                    "Could not determine cache directory for your system".to_string(),
                )
            })?;

        let cache_dir = proj_dirs.cache_dir().to_path_buf();
        fs::create_dir_all(&cache_dir)?;

        Ok(cache_dir)
    }

    /// Loads the existing configuration from disk or launches the interactive setup wizard.
    ///
    /// This is the primary bootloader for the application's state. It first checks if a
    /// `config.json` exists. If it does, it immediately deserializes and returns it. If it
    /// does *not* exist, it halts execution and uses the `dialoguer` crate to present an
    /// interactive terminal menu to the user, gathering their preferred backend and
    /// necessary API credentials before writing a new config file to disk.
    pub fn load_or_init() -> Result<Self, OnboarderError> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            return Ok(config);
        }

        println!("Welcome to Instant Onboarder!");
        println!("Let's set up your configuration.\n");

        let backends = vec![
            "watsonx.ai (IBM Cloud)",
            "Local Ollama",
            "Local Source Outline (Zero-Token Extractor)",
        ];
        let selection = Select::new()
            .with_prompt("Which AI backend would you like to use?")
            .items(&backends)
            .default(0)
            .interact()
            .map_err(|e| OnboarderError::ConfigError(format!("Failed to get user input: {}", e)))?;

        let use_local_ollama = selection == 1;
        let use_local_outline = selection == 2;

        let mut watsonx_api_key = None;
        let mut watsonx_project_id = None;

        if !use_local_ollama && !use_local_outline {
            let key: String = Input::new()
                .with_prompt("Enter your watsonx.ai API key")
                .interact_text()
                .map_err(|e| {
                    OnboarderError::ConfigError(format!("Failed to get API key: {}", e))
                })?;

            if key.trim().is_empty() {
                return Err(OnboarderError::ConfigError(
                    "API key cannot be empty".to_string(),
                ));
            }

            watsonx_api_key = Some(key.trim().to_string());

            let project_id: String = Input::new()
                .with_prompt("Enter your IBM watsonx Project ID (UUID)")
                .interact_text()
                .map_err(|e| {
                    OnboarderError::ConfigError(format!("Failed to get Project ID: {}", e))
                })?;

            if project_id.trim().is_empty() {
                return Err(OnboarderError::ConfigError(
                    "Project ID cannot be empty".to_string(),
                ));
            }
            watsonx_project_id = Some(project_id.trim().to_string());
        } else if use_local_ollama {
            println!("\nUsing local Ollama. Make sure Ollama is installed and running.");
        } else {
            println!("\nUsing Static Docs mode. AI generation disabled.");
        }

        let cache_dir = Self::default_cache_dir()?;

        let config = AppConfig {
            watsonx_api_key,
            watsonx_project_id,
            use_local_ollama,
            use_local_outline,
            cache_dir,
        };

        let json = serde_json::to_string_pretty(&config)?;
        fs::write(&config_path, json)?;

        println!("\n✓ Configuration saved to: {}", config_path.display());

        Ok(config)
    }

    /// Verifies that the loaded configuration is logically sound and ready for execution.
    ///
    /// This ensures that if the user manually tampered with the `config.json` file, the
    /// application catches missing credentials *before* attempting network requests. It also
    /// double-checks that the cache directory hasn't been manually deleted between runs.
    pub fn validate(&self) -> Result<(), OnboarderError> {
        if !self.use_local_ollama && !self.use_local_outline {
            if self.watsonx_api_key.is_none() || self.watsonx_project_id.is_none() {
                return Err(OnboarderError::ConfigError(
                    "watsonx.ai credentials are required when not using Ollama or Static mode"
                        .to_string(),
                ));
            }
        }

        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)?;
        }

        Ok(())
    }
}
