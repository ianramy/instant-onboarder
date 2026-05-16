use crate::errors::OnboarderError;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Manages caching of AI-generated explanations to save tokens
pub struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    /// Create a new CacheManager with the specified cache directory
    pub fn new(cache_dir: PathBuf) -> Result<Self, OnboarderError> {
        // Ensure cache directory exists
        fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir })
    }

    /// Get the cache directory path
    #[allow(dead_code)]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get a cached explanation for a given file hash
    ///
    /// Returns Some(content) if cached, None if not found
    pub fn get_cached_explanation(&self, file_hash: &str) -> Option<String> {
        let cache_file = self.cache_dir.join(format!("{}.md", file_hash));

        if cache_file.exists() {
            fs::read_to_string(cache_file).ok()
        } else {
            None
        }
    }

    /// Save an AI-generated explanation to the cache
    ///
    /// The content is saved as a markdown file named after the file hash
    pub fn save_explanation(&self, file_hash: &str, content: &str) -> Result<(), OnboarderError> {
        let cache_file = self.cache_dir.join(format!("{}.md", file_hash));
        fs::write(cache_file, content)?;
        Ok(())
    }

    /// Clear all cached explanations
    pub fn clear_cache(&self) -> Result<(), OnboarderError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    /// Get the number of cached files
    pub fn cache_count(&self) -> Result<usize, OnboarderError> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let count = fs::read_dir(&self.cache_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "md")
                    .unwrap_or(false)
            })
            .count();

        Ok(count)
    }
}

/// Generate a SHA-256 hash of a file's contents
///
/// This hash is used as a cache key to determine if a file has been processed before
pub fn hash_file(path: &Path) -> Result<String, OnboarderError> {
    // Read file contents
    let contents = fs::read(path)?;

    // Create SHA-256 hasher
    let mut hasher = Sha256::new();
    hasher.update(&contents);

    // Get the hash result and convert to hex string
    let result = hasher.finalize();
    let hash_hex = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok(hash_hex)
}

/// AI client for generating code explanations
pub struct AiClient {
    config: crate::config::AppConfig,
    client: reqwest::Client,
}

impl AiClient {
    /// Create a new AiClient with the given configuration
    pub fn new(config: crate::config::AppConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Generate an explanation for a file using the configured AI backend
    pub async fn explain_file(
        &self,
        _file_path: &Path,
        content: &str,
    ) -> Result<String, OnboarderError> {
        let prompt = format!(
            "You are an expert developer. Explain the purpose and architecture of the following code concisely in Markdown format:\n\n{}",
            content
        );

        if self.config.use_local_ollama {
            self.explain_with_ollama(&prompt).await
        } else if let Some(ref api_key) = self.config.watsonx_api_key {
            self.explain_with_watsonx(&prompt, api_key).await
        } else {
            Err(OnboarderError::ConfigError(
                "No valid AI backend configured. Use --setup to configure watsonx.ai or Ollama."
                    .to_string(),
            ))
        }
    }

    /// Generate explanation using local Ollama
    async fn explain_with_ollama(&self, prompt: &str) -> Result<String, OnboarderError> {
        use serde_json::json;

        let request_body = json!({
            "model": "granite-code",
            "prompt": prompt,
            "stream": false
        });

        let response = self
            .client
            .post("http://localhost:11434/api/generate")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OnboarderError::NetworkError(
                response.error_for_status().unwrap_err(),
            ));
        }

        let response_json: serde_json::Value = response.json().await?;

        let explanation = response_json["response"]
            .as_str()
            .ok_or_else(|| {
                OnboarderError::ParsingError(
                    "Failed to parse Ollama response: missing 'response' field".to_string(),
                )
            })?
            .to_string();

        Ok(explanation)
    }

    /// Generate explanation using IBM watsonx.ai
    async fn explain_with_watsonx(
        &self,
        prompt: &str,
        api_key: &str,
    ) -> Result<String, OnboarderError> {
        use serde_json::json;

        // IBM watsonx.ai API endpoint
        // Note: This is a placeholder URL - actual endpoint may vary by region
        let endpoint = "https://us-south.ml.cloud.ibm.com/ml/v1/text/generation?version=2023-05-29";

        let request_body = json!({
            "input": prompt,
            "parameters": {
                "decoding_method": "greedy",
                "max_new_tokens": 1000,
                "min_new_tokens": 1,
                "stop_sequences": [],
                "repetition_penalty": 1.0
            },
            "model_id": "ibm/granite-13b-chat-v2",
            "project_id": "your-project-id"
        });

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OnboarderError::ParsingError(format!(
                "watsonx.ai API error ({}): {}",
                status, error_text
            )));
        }

        let response_json: serde_json::Value = response.json().await?;

        let explanation = response_json["results"][0]["generated_text"]
            .as_str()
            .ok_or_else(|| {
                OnboarderError::ParsingError(
                    "Failed to parse watsonx.ai response: missing 'results[0].generated_text' field".to_string()
                )
            })?
            .to_string();

        Ok(explanation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hash_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!").unwrap();

        let hash1 = hash_file(&file_path).unwrap();
        let hash2 = hash_file(&file_path).unwrap();

        // Same content should produce same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_hash_file_different_content() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.txt");
        let file2 = temp_dir.path().join("test2.txt");

        fs::write(&file1, "Hello, World!").unwrap();
        fs::write(&file2, "Goodbye, World!").unwrap();

        let hash1 = hash_file(&file1).unwrap();
        let hash2 = hash_file(&file2).unwrap();

        // Different content should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cache_manager_save_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf()).unwrap();

        let hash = "abc123";
        let content = "# Test Explanation\n\nThis is a test.";

        // Save explanation
        cache_manager.save_explanation(hash, content).unwrap();

        // Retrieve explanation
        let retrieved = cache_manager.get_cached_explanation(hash);
        assert_eq!(retrieved, Some(content.to_string()));
    }

    #[test]
    fn test_cache_manager_get_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf()).unwrap();

        let retrieved = cache_manager.get_cached_explanation("nonexistent");
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_cache_manager_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Save some explanations
        cache_manager.save_explanation("hash1", "content1").unwrap();
        cache_manager.save_explanation("hash2", "content2").unwrap();

        assert_eq!(cache_manager.cache_count().unwrap(), 2);

        // Clear cache
        cache_manager.clear_cache().unwrap();

        assert_eq!(cache_manager.cache_count().unwrap(), 0);
    }

    #[test]
    fn test_cache_manager_count() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(cache_manager.cache_count().unwrap(), 0);

        cache_manager.save_explanation("hash1", "content1").unwrap();
        assert_eq!(cache_manager.cache_count().unwrap(), 1);

        cache_manager.save_explanation("hash2", "content2").unwrap();
        assert_eq!(cache_manager.cache_count().unwrap(), 2);
    }
}

// Made with Bob
