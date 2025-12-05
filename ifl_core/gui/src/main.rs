#![allow(non_snake_case)]
use dioxus::prelude::*;
use ifl_core::llm_client::LlmClient;
use ifl_core::{
    profile::{AnswerTags, ToneHint},
    IflCore, InputEvent,
};

fn main() {
    launch(App);
}

fn App() -> Element {
    // Global State
    let mut core = use_signal(|| IflCore::new());
    let mut session_id = use_signal(|| {
        core.read()
            .start_message()
            .unwrap_or_else(|_| "init_failed".to_string())
    });
    let mut text = use_signal(|| String::new());
    let mut messages = use_signal(|| Vec::<(String, bool)>::new());
    let mut analysis = use_signal(|| None::<AnswerTags>);

    // Handlers
    let submit_message = move |input_text: String| {
        if input_text.trim().is_empty() {
            return;
        }

        let core_ref = core.read();
        let id = session_id.read().clone();

        println!("Submitting message: id={}, text='{}'", id, input_text);

        // Push Submit
        if let Err(e) = core_ref.push_event(&id, InputEvent::Submit { ts: 0 }) {
            println!("Error pushing submit event: {}", e);
            messages
                .write()
                .push((format!("System Error: {}", e), false));
            return;
        }

        // Finalize & Analyze
        match core_ref.finalize_message(&id, &input_text) {
            Ok(json_res) => {
                match serde_json::from_str::<ifl_core::profile::InputProfile>(&json_res) {
                    Ok(profile) => {
                        analysis.set(Some(profile.tags.clone()));
                        messages.write().push((input_text.clone(), true));

                        // LLM Call
                        let tags = profile.tags.clone();
                        let prompt_text = input_text.clone();
                        spawn(async move {
                            let llm_client = LlmClient::new(None, None);
                            match llm_client.generate_response(&prompt_text, &tags).await {
                                Ok(response) => messages.write().push((response, false)),
                                Err(e) => {
                                    messages.write().push((format!("LLM Error: {}", e), false))
                                }
                            }
                        });
                    }
                    Err(e) => {
                        println!("Error parsing profile JSON: {}", e);
                        messages
                            .write()
                            .push((format!("Data Error: Failed to parse profile"), false));
                    }
                }
            }
            Err(e) => {
                println!("Error finalizing message: {}", e);
                messages
                    .write()
                    .push((format!("Analysis Error: {}", e), false));
            }
        }

        // Reset
        text.set(String::new());
        if let Ok(new_id) = core.read().start_message() {
            session_id.set(new_id);
        } else {
            messages.write().push((
                "System Error: Failed to start new session".to_string(),
                false,
            ));
        }
    };

    let handle_input = move |val: String| {
        let current_len = text.read().len();
        let new_len = val.len();
        let ts = Utc::now().timestamp_millis() as u64;
        let core_ref = core.read();
        let id = session_id.read();

        if new_len > current_len {
fn AnalysisDetails(tags: AnswerTags) -> Element {
    rsx! {
        div { class: "flex flex-col gap-4",
            div { class: "p-4 bg-gray-700 rounded-lg",
                h3 { class: "text-sm text-gray-400 uppercase", "Tone" }
                div { class: "text-2xl", "{tags.tone_hint:?}" }
            }
            div { class: "p-4 bg-gray-700 rounded-lg",
                h3 { class: "text-sm text-gray-400 uppercase", "Mode" }
                ul {
                    for mode in &tags.answer_mode {
                        li { class: "badge badge-primary", "{mode:?}" }
                    }
                }
            }
            div { class: "p-4 bg-gray-700 rounded-lg",
                h3 { class: "text-sm text-gray-400 uppercase", "Confidence" }
                div { class: "text-xl", "{tags.confidence:.2}" }
            }
        }
    }
}

#[component]
fn ChatArea(
    messages: Signal<Vec<(String, bool)>>,
    text: Signal<String>,
    on_submit: EventHandler<String>,
    on_input: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col",
            MessageList { messages: messages }
            InputArea { text: text, on_submit: on_submit, on_input: on_input }
        }
    }
}

#[component]
fn MessageList(messages: Signal<Vec<(String, bool)>>) -> Element {
    rsx! {
        div { class: "flex-1 p-4 overflow-y-auto space-y-4",
            for (msg, is_user) in messages.read().iter() {
                div { class: if *is_user { "flex justify-end" } else { "flex justify-start" },
                    div { class: if *is_user { "bg-blue-600 p-3 rounded-lg max-w-xl" } else { "bg-gray-700 p-3 rounded-lg max-w-xl" },
                        "{msg}"
                    }
                }
            }
        }
    }
}

#[component]
fn InputArea(
    text: Signal<String>,
    on_submit: EventHandler<String>,
    on_input: EventHandler<String>,
) -> Element {
    let submit = move |_| {
        let val = text.read().clone();
        on_submit.call(val);
    };

    rsx! {
        div { class: "p-4 bg-gray-800 border-t border-gray-700",
            div { class: "flex gap-2",
                input {
                    class: "flex-1 bg-gray-900 border border-gray-600 rounded p-2 text-white focus:outline-none focus:border-blue-500",
                    value: "{text}",
                    oninput: move |evt| on_input.call(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Enter && !evt.modifiers().contains(Modifiers::SHIFT) {
                            let val = text.read().clone();
                            on_submit.call(val);
                        }
                    }
                }
                button {
                    class: "bg-blue-600 hover:bg-blue-700 px-6 py-2 rounded font-bold transition",
                    onclick: submit,
                    "Send"
                }
            }
        }
    }
}
