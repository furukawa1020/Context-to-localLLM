#![allow(non_snake_case)]
use chrono::Utc;
use dioxus::prelude::*;
use ifl_core::llm_client::LlmClient;
use ifl_core::{
    profile::{AnswerTags, InputProfile, ToneHint},
    DeleteKind, IflCore, InputEvent,
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
    let mut analysis = use_signal(|| None::<ifl_core::profile::InputProfile>);

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
                        analysis.set(Some(profile.clone()));
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
            // Insert
            let diff = new_len - current_len;
            if diff > 1 {
                // Paste detected (heuristic)
                println!("Paste detected: length={}", diff);
                if let Err(e) = core_ref.push_event(&id, InputEvent::Paste { length: diff, ts }) {
                    println!("Input Error (ignored): {}", e);
                }
            } else {
                // Single char insert
                if let Some(ch) = val.chars().last() {
                    println!("Key Insert: '{}'", ch);
                    if let Err(e) = core_ref.push_event(&id, InputEvent::KeyInsert { ch, ts }) {
                        println!("Input Error (ignored): {}", e);
                    }
                }
            }
        } else if new_len < current_len {
            // Delete
            let diff = current_len - new_len;
            println!("Key Delete: count={}", diff);
            if let Err(e) = core_ref.push_event(
                &id,
                InputEvent::KeyDelete {
                    kind: DeleteKind::Backspace,
                    count: diff as u32,
                    ts,
                },
            ) {
                println!("Input Error (ignored): {}", e);
            }
        }

        text.set(val.clone());

        // Real-time Analysis Preview
        if let Ok(json_res) = core_ref.preview_message(&id, &val) {
            if let Ok(profile) = serde_json::from_str::<ifl_core::profile::InputProfile>(&json_res)
            {
                analysis.set(Some(profile));
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
fn Sidebar(analysis: Signal<Option<ifl_core::profile::InputProfile>>) -> Element {
    let system_prompt = use_memo(move || {
        if let Some(profile) = analysis.read().as_ref() {
            let client = LlmClient::new(None, None);
            client.build_system_prompt(&profile.tags)
        } else {
            "Waiting for input...".to_string()
        }
    });

    rsx! {
        div { class: "w-1/3 p-4 bg-gray-900 border-r border-blue-900 flex flex-col gap-4 overflow-y-auto font-mono",
            // Header
            div { class: "flex items-center gap-2 mb-2",
                div { class: "w-3 h-3 bg-blue-500 rounded-full animate-pulse" }
                h2 { class: "text-xl font-bold text-blue-400 tracking-widest", "IFL CORE" }
            }

            if let Some(profile) = analysis.read().as_ref() {
                // Status Badge
                div { class: "p-4 bg-gray-800/50 border border-blue-500/30 rounded-lg relative overflow-hidden",
                    div { class: "absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-blue-500 to-cyan-400" }
                    h3 { class: "text-xs text-blue-300 uppercase mb-2 tracking-wider", "User State" }
                    div { class: "flex flex-wrap gap-2",
                        for state in &profile.tags.user_state {
                            div { class: "px-3 py-1 bg-blue-500/20 border border-blue-400 text-blue-200 rounded text-sm font-bold shadow-[0_0_10px_rgba(59,130,246,0.5)] animate-pulse",
                                "{state:?}"
                            }
                        }
                        if profile.tags.user_state.is_empty() {
                            div { class: "text-gray-500 text-sm", "Analyzing..." }
                        }
                    }
                }

                // Metrics HUD
                div { class: "grid grid-cols-2 gap-3",
                    MetricCard { label: "SPEED", value: format!("{:.1}", profile.timing.avg_chars_per_sec), unit: "CPS", color: "text-cyan-400" }
                    MetricCard { label: "CONFIDENCE", value: format!("{:.0}%", profile.tags.confidence * 100.0), unit: "", color: "text-green-400" }
                    MetricCard { label: "BURSTS", value: format!("{}", profile.timing.typing_bursts), unit: "", color: "text-yellow-400" }
                    MetricCard { label: "EDITS", value: format!("{}", profile.editing.backspace_count), unit: "", color: "text-red-400" }
                }

                // Intent Analysis
                div { class: "p-4 bg-gray-800/50 border border-purple-500/30 rounded-lg",
                    h3 { class: "text-xs text-purple-300 uppercase mb-2 tracking-wider", "Detected Intent" }
                    div { class: "flex flex-wrap gap-2 mb-2",
                        for mode in &profile.tags.answer_mode {
                            span { class: "px-2 py-0.5 bg-purple-500/20 text-purple-200 text-xs rounded border border-purple-500/30", "{mode:?}" }
                        }
                    }
                    div { class: "flex justify-between text-xs text-gray-400",
                        span { "Tone: {profile.tags.tone_hint:?}" }
                        span { "Depth: {profile.tags.depth_hint:?}" }
                    }
                }

                // System Prompt Preview (Terminal Style)
                div { class: "p-4 bg-black border border-green-500/30 rounded-lg font-mono text-xs relative",
                    div { class: "absolute top-2 right-2 w-2 h-2 bg-green-500 rounded-full animate-ping" }
                    h3 { class: "text-green-600 uppercase mb-2 tracking-wider border-b border-green-900 pb-1", "System Prompt" }
                    div { class: "text-green-400 whitespace-pre-wrap opacity-80 h-32 overflow-y-auto custom-scrollbar",
                        "{system_prompt}"
                    }
                }

                // Raw Data Toggle
                details { class: "group",
                    summary { class: "cursor-pointer text-xs text-gray-500 hover:text-blue-300 transition-colors list-none flex items-center gap-2",
                        span { class: "w-1 h-1 bg-gray-500 rounded-full group-open:bg-blue-400" }
                        "RAW DATA STREAM"
                    }
                    div { class: "mt-2 text-[10px] font-mono bg-black/50 p-2 rounded text-gray-400 whitespace-pre-wrap overflow-x-auto border border-gray-800",
                        "{serde_json::to_string_pretty(profile).unwrap_or_default()}"
                    }
                }

            } else {
                div { class: "flex flex-col items-center justify-center h-64 text-gray-600 gap-4",
                    div { class: "w-16 h-16 border-4 border-gray-700 border-t-blue-500 rounded-full animate-spin" }
                    div { "AWAITING INPUT SIGNAL..." }
                }
            }
        }
    }
}

#[component]
fn MetricCard(label: String, value: String, unit: String, color: String) -> Element {
    rsx! {
        div { class: "bg-gray-800/50 p-3 rounded border border-gray-700 flex flex-col items-center justify-center",
            div { class: "text-[10px] text-gray-500 uppercase tracking-widest mb-1", "{label}" }
            div { class: "text-2xl font-bold {color} font-mono", "{value}" }
            if !unit.is_empty() {
                div { class: "text-[10px] text-gray-600", "{unit}" }
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
