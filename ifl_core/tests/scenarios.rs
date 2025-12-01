use ifl_core::profile::{AnswerMode, SourceType};
use ifl_core::{IflCore, InputEvent};

#[test]
fn test_scenario_summarize_paste() {
    let core = IflCore::new();
    let id = core.start_message();

    // Simulate typing "Check this out:"
    let mut ts = 1000;
    for ch in "Check this out:".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Simulate pasting a large block
    core.push_event(&id, InputEvent::Paste { length: 500, ts })
        .unwrap();
    ts += 500;

    // Submit
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Finalize with text that looks like a pasted article
    let final_text = "Check this out:\n\n".to_string() + &"A long article content... ".repeat(20);

    let json = core.finalize_message(&id, &final_text).unwrap();
    println!("JSON: {}", json);

    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // Verify
    assert!(profile.source.paste_ratio > 0.8); // 500 pasted vs ~15 typed
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Summarize));
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Structure));
}

#[test]
fn test_scenario_refine_typing() {
    let core = IflCore::new();
    let id = core.start_message();

    let mut ts = 1000;

    // Type a lot over a long time
    for _ in 0..50 {
        core.push_event(&id, InputEvent::KeyInsert { ch: 'a', ts })
            .unwrap();
        ts += 1000; // Slow typing, total 50s
    }

    // Backspace a lot
    for _ in 0..25 {
        core.push_event(
            &id,
            InputEvent::KeyDelete {
                kind: ifl_core::event::DeleteKind::Backspace,
                count: 1,
                ts,
            },
        )
        .unwrap();
        ts += 200;
    }

    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let final_text = "Final polished thought.";
    let json = core.finalize_message(&id, final_text).unwrap();

    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // Verify
    assert_eq!(profile.source.source_type, SourceType::TypedOnly);
    assert!(profile.timing.total_duration_ms > 30_000);
    assert!(profile.editing.backspace_count > 20);
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Refine));
}

#[test]
fn test_scenario_japanese_summary() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Simulate typing Japanese request
    let text = "これは議事録です。要約してください。";
    for ch in text.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 200;
    }

    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, text).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert!(profile.structure.japanese_detected);
    assert!(profile.structure.request_summary);
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Summarize));
}
