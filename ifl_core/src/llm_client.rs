use crate::profile::{AnswerMode, AnswerTags, ToneHint};
use reqwest::Client;
use serde_json::json;
use std::error::Error;

pub struct LlmClient {
    client: Client,
    base_url: String,
    model: String,
}

impl LlmClient {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url
                .unwrap_or_else(|| "http://localhost:11434/v1/chat/completions".to_string()),
            model: model.unwrap_or_else(|| "llama3.2:3b".to_string()), // Default to llama3.2:3b
        }
    }

    pub async fn generate_response(
        &self,
        text: &str,
        analysis: &AnswerTags,
    ) -> Result<String, Box<dyn Error>> {
        let system_prompt = self.build_system_prompt(analysis);

        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ],
            "stream": false
        });

        let res = self.client.post(&self.base_url).json(&body).send().await?;

        if !res.status().is_success() {
            return Err(format!("API request failed with status: {}", res.status()).into());
        }

        let json_res: serde_json::Value = res.json().await?;

        // Extract content from OpenAI-compatible response
        let content = json_res["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to parse response content")?
            .to_string();

        Ok(content)
    }

    fn build_system_prompt(&self, analysis: &AnswerTags) -> String {
        let mut prompt = String::from("You are a helpful AI assistant.");

        // Add tone instruction
        match analysis.tone_hint {
            ToneHint::Direct => prompt.push_str(" Be direct and concise."),
            ToneHint::Gentle => prompt.push_str(" Be polite and gentle."),
            ToneHint::Neutral => {}
        }

        // Add mode instructions
        if !analysis.answer_mode.is_empty() {
            prompt.push_str("\n\nFollow these guidelines based on the user's input pattern:");
            for mode in &analysis.answer_mode {
                match mode {
                    AnswerMode::Summarize => prompt.push_str("\n- Summarize the input text."),
                    AnswerMode::Structure => prompt.push_str("\n- Structure the content with bullet points or headers."),
                    AnswerMode::Refine => prompt.push_str("\n- Refine and polish the text for better clarity."),
                    AnswerMode::ClarifyQuestion => prompt.push_str("\n- The user seems to be asking a question or needs clarification. Answer it clearly."),
                    AnswerMode::Explore => prompt.push_str("\n- Explore the topic further and provide related information."),
                    AnswerMode::Complete => prompt.push_str("\n- Complete the user's sentence or code."),
                    _ => {},
                }
            }
        }

        prompt
    }
}
