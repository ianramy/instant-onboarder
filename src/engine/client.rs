//! AI Client handling generation requests to remote or local language models.
//!
//! This module acts as the networking abstraction layer for the application. It takes
//! raw source code strings, wraps them in strict system prompts, and routes the request
//! to the appropriate AI backend based on the user's active configuration (e.g., local
//! extraction, local Ollama, or remote IBM watsonx.ai).

use crate::config::AppConfig;
use crate::engine::extractor::extract_outline;
use crate::errors::OnboarderError;
use reqwest::Client;
use std::path::Path;

/// High-level client for dispatching analysis requests to configured AI backends.
///
/// The `AiClient` holds the application's configuration state and a reusable asynchronous
/// HTTP client pool (`reqwest::Client`). It handles authentication handshakes, payload
/// formatting, and JSON response parsing.
pub struct AiClient {
    /// The global application configuration, dictating which AI backend is currently active.
    pub config: AppConfig,
    /// An asynchronous HTTP client optimized for connection pooling across multiple requests.
    pub client: Client,
}

impl AiClient {
    /// Instantiates a new `AiClient` utilizing the provided application configuration.
    ///
    /// It initializes a fresh, default `reqwest::Client` under the hood. Creating a single
    /// client and reusing it across requests is highly recommended to leverage internal
    /// connection pooling and keep-alive features.
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Primary entry point to generate a Markdown explanation for a specific source file.
    ///
    /// This function acts as a router. It evaluates the user's configuration flags and
    /// delegates the raw `content` to the appropriate handler. If the zero-token local
    /// outline feature is enabled, it completely bypasses network calls and instantly
    /// returns a lexically parsed summary.
    pub async fn explain_file(
        &self,
        _file_path: &Path,
        content: &str,
    ) -> Result<String, OnboarderError> {
        if self.config.use_local_outline {
            return Ok(extract_outline(content));
        }

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

    /// Dispatches a generation request to a locally running Ollama instance.
    ///
    /// This method assumes Ollama is running on the default local port (`localhost:11434`).
    /// It utilizes the `granite-code` model to generate explanations. The request is made
    /// synchronously from the LLM's perspective (stream = false) to capture the entire
    /// response payload at once before handing it back to the TUI.
    pub async fn explain_with_ollama(&self, prompt: &str) -> Result<String, OnboarderError> {
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

    /// Exchanges an IBM Cloud API Key for a short-lived IAM Bearer token.
    ///
    /// IBM Watsonx requires an active Identity and Access Management (IAM) token for all
    /// API requests, rather than a raw API key. This helper function executes a secure
    /// OAuth handshake via URL-encoded form data to dynamically retrieve that token.
    pub async fn get_iam_token(&self, api_key: &str) -> Result<String, OnboarderError> {
        use std::collections::HashMap;

        let mut params = HashMap::new();
        params.insert("grant_type", "urn:ibm:params:oauth:grant-type:apikey");
        params.insert("apikey", api_key);

        let response = self
            .client
            .post("https://iam.cloud.ibm.com/identity/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(OnboarderError::NetworkError)?;

        if !response.status().is_success() {
            return Err(OnboarderError::ConfigError(
                "Failed to authenticate with IBM Cloud. Check your API Key.".to_string(),
            ));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            OnboarderError::ParsingError(format!("Failed to parse IAM response: {}", e))
        })?;

        let access_token = response_json["access_token"]
            .as_str()
            .ok_or_else(|| {
                OnboarderError::ParsingError("No access_token found in IAM response".to_string())
            })?
            .to_string();

        Ok(access_token)
    }

    /// Dispatches a generation request to the remote IBM watsonx.ai text generation endpoint.
    ///
    /// This method leverages the `ibm-granite/granite-4.0-h-small` model. It handles the
    /// two-step authentication process by first acquiring an IAM token, then injecting it
    /// into the Authorization header alongside the specific Project ID required by Watsonx.
    /// It uses greedy decoding logic to ensure fast, deterministic architectural summaries.
    pub async fn explain_with_watsonx(
        &self,
        prompt: &str,
        api_key: &str,
    ) -> Result<String, OnboarderError> {
        use serde_json::json;

        let project_id = self.config.watsonx_project_id.as_ref().ok_or_else(|| {
            OnboarderError::ConfigError(
                "Watsonx Project ID is missing from configuration. Run with --setup to reconfigure."
                    .to_string(),
            )
        })?;
        let iam_token = self.get_iam_token(api_key).await?;

        let endpoint = "https://us-south.ml.cloud.ibm.com/ml/v1/text/generation?version=2023-05-29";

        let request_body = json!({
            "input": prompt,
            "parameters": {
                "decoding_method": "greedy",
                "max_new_tokens": 500,
                "min_new_tokens": 1,
                "stop_sequences": [],
                "repetition_penalty": 1.0
            },
            "model_id": "ibm-granite/granite-4.0-h-small",
            "project_id": project_id
        });

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", iam_token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(OnboarderError::NetworkError)?;

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

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OnboarderError::ParsingError(format!("Failed to parse JSON: {}", e)))?;

        let explanation = response_json["results"][0]["generated_text"]
            .as_str()
            .ok_or_else(|| {
                OnboarderError::ParsingError(
                    "Failed to parse watsonx.ai response: missing 'results[0].generated_text'"
                        .to_string(),
                )
            })?
            .to_string();

        Ok(explanation)
    }
}
