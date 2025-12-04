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
    let mut session_id = use_signal(|| core.read().start_message());
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
        session_id.set(core.read().start_message());
    };

    let handle_input = move |val: String| {
        text.set(val.clone());
        if let Some(ch) = val.chars().last() {
            let core_ref = core.read();
            let id = session_id.read();
            if let Err(e) = core_ref.push_event(&id, InputEvent::KeyInsert { ch, ts: 0 }) {
                println!("Input Error (ignored): {}", e);
            }
        }
    };

    rsx! {
        div { class: "flex h-screen bg-gray-900 text-white font-sans",
            // Tailwind
            script { src: "https://cdn.tailwindcss.com" }

            Sidebar { analysis: analysis }
            ChatArea {
                messages: messages,
                text: text,
                on_submit: submit_message,
                on_input: handle_input
            }
        }
    }
}

#[component]
fn Sidebar(analysis: Signal<Option<AnswerTags>>) -> Element {
    rsx! {
        div { class: "w-1/4 p-4 bg-gray-800 border-r border-gray-700 flex flex-col gap-4",
            h2 { class: "text-xl font-bold mb-4 text-blue-400", "IFL Analysis" }
            if let Some(tags) = analysis.read().as_ref() {
                AnalysisDetails { tags: tags.clone() }
            } else {
                div { class: "text-gray-500 italic", "Waiting for input..." }
            }
        }
    }
}

#[component]
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
    let submit = move |_| on_submit.call(text.read().clone());

    rsx! {
        div { class: "p-4 bg-gray-800 border-t border-gray-700",
            div { class: "flex gap-2",
                input {
                    class: "flex-1 bg-gray-900 border border-gray-600 rounded p-2 text-white focus:outline-none focus:border-blue-500",
                    value: "{text}",
                    oninput: move |evt| on_input.call(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Enter && !evt.modifiers().contains(Modifiers::SHIFT) {
                            on_submit.call(text.read().clone());
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
