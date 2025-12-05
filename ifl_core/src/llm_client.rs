use crate::profile::{AnswerMode, AnswerTags, ToneHint, UserState};
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

    pub fn build_system_prompt(&self, analysis: &AnswerTags) -> String {
        let mut prompt =
            String::from("You are an intelligent assistant analyzing user input behavior.\n");
        prompt.push_str(
            "Based on the following analysis of the user's input, adjust your response:\n\n",
        );

        prompt.push_str(&format!("- Tone: {:?}\n", analysis.tone_hint));
        prompt.push_str(&format!("- Depth: {:?}\n", analysis.depth_hint));
        prompt.push_str(&format!("- Scope: {:?}\n", analysis.scope_hint));
        prompt.push_str(&format!("- Modes: {:?}\n", analysis.answer_mode));
        prompt.push_str(&format!("- User State: {:?}\n", analysis.user_state));
        prompt.push_str(&format!("- Confidence: {:.2}\n\n", analysis.confidence));

        prompt.push_str("Guidelines:\n");
        prompt.push_str("CRITICAL: You MUST adapt your persona based on the 'User State' above.\n");
        prompt.push_str("- If 'Hesitant': Be encouraging, patient, and ask clarifying questions. Acknowledge their hesitation (e.g., 'Take your time', 'I see you're thinking carefully').\n");
        prompt.push_str(
            "- If 'Flowing': Be brief, efficient, and match their speed. Skip pleasantries.\n",
        );
        prompt.push_str("- If 'Editing': Focus on precision and detail. They are refining their thought, so you should be precise.\n");
        prompt.push_str("- If 'Scattered': Help organize their thoughts. Offer structure.\n");
        prompt.push_str(
            "- If 'Pasting': Assume they want code analysis or summarization. Be analytical.\n",
        );

        // Add mode instructions
        if !analysis.answer_mode.is_empty() {
            prompt.push_str("\nSpecific Goals:\n");
            for mode in &analysis.answer_mode {
                match mode {
                    AnswerMode::Summarize => prompt.push_str("- Summarize the input text.\n"),
                    AnswerMode::Structure => prompt.push_str("- Structure the content with bullet points or headers.\n"),
                    AnswerMode::Refine => prompt.push_str("- Refine and polish the text for better clarity.\n"),
                    AnswerMode::ClarifyQuestion => prompt.push_str("- The user seems to be asking a question or needs clarification. Answer it clearly.\n"),
                    AnswerMode::Explore => prompt.push_str("- Explore the topic further and provide related information.\n"),
                    AnswerMode::Complete => prompt.push_str("- Complete the user's sentence or code.\n"),
                }
            }
        }

        prompt
    }
}
