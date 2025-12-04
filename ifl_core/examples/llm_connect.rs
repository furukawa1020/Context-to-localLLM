use ifl_core::llm_client::LlmClient;
use ifl_core::{IflCore, InputEvent};
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() {
    // 1. Initialize Core and LLM Client
    let core = IflCore::new();
    // Assuming local LLM is running at default URL.
    // If you use a different model or URL, change it here.
    let llm_client = LlmClient::new(None, None);

    // 2. Start Session
    let session_id = core.start_message();
    println!("Session started: {}", session_id);

    // 3. Simulate Input: "Can you summarize this text?" (implies Summarize mode)
    let mut current_ts = 1000;
    let text = "Can you summarize this text? Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.";

    for ch in text.chars() {
        if let Err(e) = core.push_event(&session_id, InputEvent::KeyInsert { ch, ts: current_ts }) {
            eprintln!("Error pushing event: {}", e);
            return;
        }
        current_ts += 50; // Fast typing
    }

    // Submit
    println!("Submitting...");
    if let Err(e) = core.push_event(&session_id, InputEvent::Submit { ts: current_ts }) {
        eprintln!("Error submitting: {}", e);
        return;
    }

    // 4. Get Analysis
    println!("Finalizing message...");
    let json_result = match core.finalize_message(&session_id, text) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error finalizing message: {}", e);
            return;
        }
    };

    println!("Analysis Result: {}", json_result);

    let profile: ifl_core::profile::InputProfile = match serde_json::from_str(&json_result) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
            return;
        }
    };
    println!("Analysis: {:?}", profile.tags);

    // 5. Call LLM
    println!("Sending to LLM...");
    match llm_client.generate_response(text, &profile.tags).await {
        Ok(response) => println!("LLM Response:\n{}", response),
        Err(e) => println!("Error calling LLM: {}", e),
    }
}
