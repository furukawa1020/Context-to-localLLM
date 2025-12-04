#![allow(non_snake_case)]
use dioxus::prelude::*;
use ifl_core::llm_client::LlmClient;
use ifl_core::{
    profile::{AnswerTags, ToneHint},
    IflCore, InputEvent,
};
use std::rc::Rc;

fn main() {
    dioxus::desktop::launch(App);
}

fn App() -> Element {
    // Use signals for state in Dioxus 0.5
    let mut core = use_signal(|| IflCore::new());
    let mut session_id = use_signal(|| core.read().start_message());
    let mut text = use_signal(|| String::new());
    let mut messages = use_signal(|| Vec::<(String, bool)>::new()); // (Content, IsUser)
    let mut analysis = use_signal(|| None::<AnswerTags>);

    let submit_message = move |input_text: String| {
        let core_ref = core.read();
        let id = session_id.read();

        // Push Submit event
        core_ref
            .push_event(&id, InputEvent::Submit { ts: 0 })
            .unwrap();

        // Finalize
        let json_res = core_ref.finalize_message(&id, &input_text).unwrap();
        let profile: ifl_core::profile::InputProfile = serde_json::from_str(&json_res).unwrap();

        analysis.set(Some(profile.tags.clone()));

        // Add user message
        messages.write().push((input_text.clone(), true));

        // Call LLM
        let tags = profile.tags.clone();

        // Spawn async task
        spawn(async move {
            let llm_client = LlmClient::new(None, None);
            match llm_client.generate_response(&input_text, &tags).await {
                Ok(response) => {
                    messages.write().push((response, false));
                }
                Err(e) => {
                    messages.write().push((format!("Error: {}", e), false));
                }
            }
        });

        // Reset text and start new session
        text.set(String::new());
        session_id.set(core.read().start_message());
    };

    let handle_input = move |evt: FormEvent| {
        let val = evt.value();
        text.set(val.clone());

        if let Some(ch) = val.chars().last() {
            core.read()
                .push_event(&session_id.read(), InputEvent::KeyInsert { ch, ts: 0 })
                .unwrap();
        }
    };

    rsx! {
        // Tailwind CDN
        head::script { src: "https://cdn.tailwindcss.com" }

        div { class: "flex h-screen bg-gray-900 text-white font-sans",
            // Sidebar / Dashboard
            div { class: "w-1/4 p-4 bg-gray-800 border-r border-gray-700 flex flex-col gap-4",
                h2 { class: "text-xl font-bold mb-4 text-blue-400", "IFL Analysis" }

                if let Some(tags) = analysis.read().as_ref() {
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
                    for (msg, is_user) in messages.read().iter() {
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
                                    submit_message(text.read().clone());
                                }
                            }
                        }
                        button {
                            class: "bg-blue-600 hover:bg-blue-700 px-6 py-2 rounded font-bold transition",
                            onclick: move |_| submit_message(text.read().clone()),
                            "Send"
                        }
                    }
                }
            }
        }
    }
}
