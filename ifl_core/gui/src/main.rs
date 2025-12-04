#![allow(non_snake_case)]
use dioxus::prelude::*;
use ifl_core::llm_client::LlmClient;
use ifl_core::{
    profile::{AnswerMode, AnswerTags, ToneHint},
    IflCore, InputEvent,
};
use std::cell::RefCell;
use std::rc::Rc;
use tokio::runtime::Runtime;

fn main() {
    // Initialize Core and Client
    // We need to keep them alive. For Dioxus Desktop, we can use global state or pass them down.
    // Since `IflCore` uses Arc<Mutex>, it's cloneable.
    // `LlmClient` is not cloneable by default, let's wrap it or recreate it.
    // Ideally we wrap LlmClient in Arc.

    dioxus::desktop::launch(App);
}

#[derive(Clone)]
struct AppState {
    core: IflCore,
    llm_client: Rc<LlmClient>, // Rc for single thread (Dioxus runs on main thread usually, but async tasks might need Send. LlmClient uses reqwest which is Send. So Arc is better)
                               // Actually, let's just create LlmClient when needed or wrap in Arc.
                               // But `LlmClient` struct definition in `ifl_core` might not be Clone.
                               // Let's assume we can create a new one or it's cheap.
}

fn App(cx: Scope) -> Element {
    let core = use_state(cx, || IflCore::new());
    let session_id = use_state(cx, || core.get().start_message());
    let text = use_state(cx, || String::new());
    let messages = use_state(cx, || Vec::<(String, bool)>::new()); // (Content, IsUser)
    let analysis = use_state(cx, || None::<AnswerTags>);

    let submit_message = move |input_text: String| {
        let core_ref = core.get();
        let id = session_id.get();

        // Push Submit event
        // We should have been pushing KeyInsert events during typing.
        // For simplicity in this demo, we might just push all events now or assume they were pushed.
        // Let's implement real-time event pushing in the input handler.

        core_ref
            .push_event(id, InputEvent::Submit { ts: 0 })
            .unwrap(); // Dummy TS

        // Finalize
        let json_res = core_ref.finalize_message(id, &input_text).unwrap();
        let profile: ifl_core::profile::InputProfile = serde_json::from_str(&json_res).unwrap();

        analysis.set(Some(profile.tags.clone()));

        // Add user message
        let mut new_msgs = messages.get().clone();
        new_msgs.push((input_text.clone(), true));
        messages.set(new_msgs);

        // Call LLM
        let llm_client = LlmClient::new(None, None);
        let tags = profile.tags.clone();
        let msgs_handle = messages.clone();

        cx.spawn(async move {
            match llm_client.generate_response(&input_text, &tags).await {
                Ok(response) => {
                    let mut current_msgs = msgs_handle.get().clone();
                    current_msgs.push((response, false));
                    msgs_handle.set(current_msgs);
                }
                Err(e) => {
                    let mut current_msgs = msgs_handle.get().clone();
                    current_msgs.push((format!("Error: {}", e), false));
                    msgs_handle.set(current_msgs);
                }
            }
        });

        // Reset text and start new session?
        // Usually chat keeps history. `ifl_core` is per-message analysis.
        // So we start a NEW session for the next message.
        text.set(String::new());
        session_id.set(core_ref.start_message());
    };

    let handle_input = move |evt: FormEvent| {
        let val = evt.value.clone();
        text.set(val.clone());

        // Push KeyInsert (simplified, just pushing last char or assuming paste if long)
        // Real implementation would diff or capture keydown.
        // For now, let's just update text. Real-time analysis needs events.
        // Let's simulate "typing" event for the last char.
        if let Some(ch) = val.chars().last() {
            core.get()
                .push_event(session_id.get(), InputEvent::KeyInsert { ch, ts: 0 })
                .unwrap();
        }
    };

    render! {
        // Tailwind CDN
        head {
            script { src: "https://cdn.tailwindcss.com" }
        }

        div { class: "flex h-screen bg-gray-900 text-white font-sans",
            // Sidebar / Dashboard
            div { class: "w-1/4 p-4 bg-gray-800 border-r border-gray-700 flex flex-col gap-4",
                h2 { class: "text-xl font-bold mb-4 text-blue-400", "IFL Analysis" }

                if let Some(tags) = analysis.get() {
                    div { class: "p-4 bg-gray-700 rounded-lg",
                        h3 { class: "text-sm text-gray-400 uppercase", "Tone" }
                        div { class: "text-2xl", "{:?}", tags.tone_hint }
                    }
                    div { class: "p-4 bg-gray-700 rounded-lg",
                        h3 { class: "text-sm text-gray-400 uppercase", "Mode" }
                        ul {
                            for mode in &tags.answer_mode {
                                li { class: "badge badge-primary", "{:?}", mode }
                            }
                        }
                    }
                    div { class: "p-4 bg-gray-700 rounded-lg",
                        h3 { class: "text-sm text-gray-400 uppercase", "Confidence" }
                        div { class: "text-xl", "{:.2}", tags.confidence }
                    }
                } else {
                    div { class: "text-gray-500 italic", "Waiting for input..." }
                }
            }

            // Chat Area
            div { class: "flex-1 flex flex-col",
                // Messages
                div { class: "flex-1 p-4 overflow-y-auto space-y-4",
                    for (msg, is_user) in messages.get() {
                        div { class: if *is_user { "flex justify-end" } else { "flex justify-start" },
                            div { class: if *is_user { "bg-blue-600 p-3 rounded-lg max-w-xl" } else { "bg-gray-700 p-3 rounded-lg max-w-xl" },
                                "{msg}"
                            }
                        }
                    }
                }

                // Input
                div { class: "p-4 bg-gray-800 border-t border-gray-700",
                    div { class: "flex gap-2",
                        input {
                            class: "flex-1 bg-gray-900 border border-gray-600 rounded p-2 text-white focus:outline-none focus:border-blue-500",
                            value: "{text}",
                            oninput: handle_input,
                            onkeydown: move |evt| {
                                if evt.key() == Key::Enter && !evt.modifiers().contains(Modifiers::SHIFT) {
                                    submit_message(text.get().clone());
                                }
                            }
                        }
                        button {
                            class: "bg-blue-600 hover:bg-blue-700 px-6 py-2 rounded font-bold transition",
                            onclick: move |_| submit_message(text.get().clone()),
                            "Send"
                        }
                    }
                }
            }
        }
    }
}
