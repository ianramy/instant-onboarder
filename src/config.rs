use crate::errors::OnboarderError;
use dialoguer::{Input, Select};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub watsonx_api_key: Option<String>,
    pub use_local_ollama: bool,
    pub cache_dir: PathBuf,
}

impl AppConfig {
    /// Get the default configuration file path
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

    /// Get the default cache directory path
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

    /// Load configuration from disk or initialize with user prompts
    pub fn load_or_init() -> Result<Self, OnboarderError> {
        let config_path = Self::config_path()?;

        // Try to load existing config
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            return Ok(config);
        }

        // Config doesn't exist, prompt user to create it
        println!("Welcome to Instant Onboarder!");
        println!("Let's set up your configuration.\n");

        // Ask which AI backend to use
        let backends = vec!["watsonx.ai (IBM Cloud)", "Local Ollama"];
        let selection = Select::new()
            .with_prompt("Which AI backend would you like to use?")
            .items(&backends)
            .default(0)
            .interact()
            .map_err(|e| OnboarderError::ConfigError(format!("Failed to get user input: {}", e)))?;

        let use_local_ollama = selection == 1;
        let mut watsonx_api_key = None;

        if !use_local_ollama {
            // Prompt for watsonx.ai API key
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
        } else {
            println!("\nUsing local Ollama. Make sure Ollama is installed and running.");
            println!("Visit https://ollama.ai for installation instructions.\n");
        }

        // Get cache directory
        let cache_dir = Self::default_cache_dir()?;

        let config = AppConfig {
            watsonx_api_key,
            use_local_ollama,
            cache_dir,
        };

        // Save config to disk
        let json = serde_json::to_string_pretty(&config)?;
        fs::write(&config_path, json)?;

        println!("\n✓ Configuration saved to: {}", config_path.display());

        Ok(config)
    }

    /// Validate that the configuration is complete and usable
    pub fn validate(&self) -> Result<(), OnboarderError> {
        if !self.use_local_ollama && self.watsonx_api_key.is_none() {
            return Err(OnboarderError::ConfigError(
                "watsonx.ai API key is required when not using local Ollama".to_string(),
            ));
        }

        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)?;
        }

        Ok(())
    }
}

// Made with Bob
