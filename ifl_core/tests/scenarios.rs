use ifl_core::profile::{AnswerMode, SourceType, ToneHint};
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

#[test]
fn test_scenario_selection_replace() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello"
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Select "Hello" (0 to 5)
    core.push_event(
        &id,
        InputEvent::SelectionChange {
            start: 0,
            end: 5,
            ts,
        },
    )
    .unwrap();
    ts += 500;

    // Type "Hi" (replacing selection)
    // First char 'H' replaces selection
    core.push_event(&id, InputEvent::KeyInsert { ch: 'H', ts })
        .unwrap();
    ts += 100;
    // Second char 'i' is normal typing
    core.push_event(&id, InputEvent::KeyInsert { ch: 'i', ts })
        .unwrap();
    ts += 100;

    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, "Hi").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert!(profile.editing.selection_edit_count >= 1);
}

#[test]
fn test_scenario_japanese_tone() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Polite
    let text_polite = "お願いします。";
    for ch in text_polite.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();
    let json = core.finalize_message(&id, text_polite).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // ToneHint::Gentle is expected for "masu/desu/kudasai"
    assert!(matches!(profile.tags.tone_hint, ToneHint::Gentle));

    // Direct
    let id2 = core.start_message();
    let text_direct = "これをやれ。";
    let text_direct_2 = "これは重要だ。";
    for ch in text_direct_2.chars() {
        core.push_event(&id2, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id2, InputEvent::Submit { ts }).unwrap();
    let json2 = core.finalize_message(&id2, text_direct_2).unwrap();
    let profile2: ifl_core::InputProfile = serde_json::from_str(&json2).unwrap();

    assert!(matches!(profile2.tags.tone_hint, ToneHint::Direct));
}

#[test]
fn test_persistence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello"
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Export
    let events_json = core.export_events(&id).unwrap();
    println!("Exported events: {}", events_json);

    // Import into new core
    let core2 = IflCore::new();
    let id2 = core2.import_events(&events_json).unwrap();

    // Finalize imported session
    let json = core2.finalize_message(&id2, "Hello").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(profile.source.source_type, SourceType::TypedOnly);
    assert_eq!(profile.structure.char_count, 5);
}

#[test]
fn test_confidence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Explicit request "Summarize this"
    let text = "Summarize this article.";
    for ch in text.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, text).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // Should have high confidence due to explicit request
    assert!(profile.tags.confidence > 0.7);
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Summarize));
}

#[test]
fn test_efficiency_score() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello" (5 chars)
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Backspace 2 chars
    for _ in 0..2 {
        core.push_event(
            &id,
            InputEvent::KeyDelete {
                kind: ifl_core::event::DeleteKind::Backspace,
                count: 1,
                ts,
            },
        )
        .unwrap();
        ts += 100;
    }

    // Type "p!" (2 chars) -> "Help!"
    for ch in "p!".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Final text "Help!" (5 chars)
    // Total typed: 5 (Hello) + 2 (p!) = 7 chars
    // Efficiency = 5 / 7 = ~0.71

    let json = core.finalize_message(&id, "Help!").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    println!("Efficiency: {}", profile.editing.efficiency_score);
    assert!(profile.editing.efficiency_score > 0.7 && profile.editing.efficiency_score < 0.72);
}

#[test]
fn test_snapshot_persistence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Snap"
    for ch in "Snap".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Export snapshot
    let snapshot_json = core.export_snapshot(&id, "Snap").unwrap();
    println!("Snapshot: {}", snapshot_json);

    let snapshot: ifl_core::profile::SessionSnapshot =
        serde_json::from_str(&snapshot_json).unwrap();

    let mut ts = 1000;

    // Type "Hello"
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Select "Hello" (0 to 5)
    core.push_event(
        &id,
        InputEvent::SelectionChange {
            start: 0,
            end: 5,
            ts,
        },
    )
    .unwrap();
    ts += 500;

    // Type "Hi" (replacing selection)
    // First char 'H' replaces selection
    core.push_event(&id, InputEvent::KeyInsert { ch: 'H', ts })
        .unwrap();
    ts += 100;
    // Second char 'i' is normal typing
    core.push_event(&id, InputEvent::KeyInsert { ch: 'i', ts })
        .unwrap();
    ts += 100;

    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, "Hi").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert!(profile.editing.selection_edit_count >= 1);
}

#[test]
fn test_scenario_japanese_tone() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Polite
    let text_polite = "お願いします。";
    for ch in text_polite.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();
    let json = core.finalize_message(&id, text_polite).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // ToneHint::Gentle is expected for "masu/desu/kudasai"
    assert!(matches!(profile.tags.tone_hint, ToneHint::Gentle));

    // Direct
    let id2 = core.start_message();
    let text_direct = "これをやれ。";
    let text_direct_2 = "これは重要だ。";
    for ch in text_direct_2.chars() {
        core.push_event(&id2, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id2, InputEvent::Submit { ts }).unwrap();
    let json2 = core.finalize_message(&id2, text_direct_2).unwrap();
    let profile2: ifl_core::InputProfile = serde_json::from_str(&json2).unwrap();

    assert!(matches!(profile2.tags.tone_hint, ToneHint::Direct));
}

#[test]
fn test_persistence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello"
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Export
    let events_json = core.export_events(&id).unwrap();
    println!("Exported events: {}", events_json);

    // Import into new core
    let core2 = IflCore::new();
    let id2 = core2.import_events(&events_json).unwrap();

    // Finalize imported session
    let json = core2.finalize_message(&id2, "Hello").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(profile.source.source_type, SourceType::TypedOnly);
    assert_eq!(profile.structure.char_count, 5);
}

#[test]
fn test_confidence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Explicit request "Summarize this"
    let text = "Summarize this article.";
    for ch in text.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, text).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // Should have high confidence due to explicit request
    assert!(profile.tags.confidence > 0.7);
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Summarize));
}

#[test]
fn test_efficiency_score() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello" (5 chars)
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Backspace 2 chars
    for _ in 0..2 {
        core.push_event(
            &id,
            InputEvent::KeyDelete {
                kind: ifl_core::event::DeleteKind::Backspace,
                count: 1,
                ts,
            },
        )
        .unwrap();
        ts += 100;
    }

    // Type "p!" (2 chars) -> "Help!"
    for ch in "p!".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Final text "Help!" (5 chars)
    // Total typed: 5 (Hello) + 2 (p!) = 7 chars
    // Efficiency = 5 / 7 = ~0.71

    let json = core.finalize_message(&id, "Help!").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    println!("Efficiency: {}", profile.editing.efficiency_score);
    assert!(profile.editing.efficiency_score > 0.7 && profile.editing.efficiency_score < 0.72);
}

#[test]
fn test_snapshot_persistence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Snap"
    for ch in "Snap".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Export snapshot
    let snapshot_json = core.export_snapshot(&id, "Snap").unwrap();
    println!("Snapshot: {}", snapshot_json);

    let snapshot: ifl_core::profile::SessionSnapshot =
        serde_json::from_str(&snapshot_json).unwrap();

    let mut ts = 1000;

    // Type "Hello"
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Select "Hello" (0 to 5)
    core.push_event(
        &id,
        InputEvent::SelectionChange {
            start: 0,
            end: 5,
            ts,
        },
    )
    .unwrap();
    ts += 500;

    // Type "Hi" (replacing selection)
    // First char 'H' replaces selection
    core.push_event(&id, InputEvent::KeyInsert { ch: 'H', ts })
        .unwrap();
    ts += 100;
    // Second char 'i' is normal typing
    core.push_event(&id, InputEvent::KeyInsert { ch: 'i', ts })
        .unwrap();
    ts += 100;

    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, "Hi").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert!(profile.editing.selection_edit_count >= 1);
}

#[test]
fn test_scenario_japanese_tone() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Polite
    let text_polite = "お願いします。";
    for ch in text_polite.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();
    let json = core.finalize_message(&id, text_polite).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // ToneHint::Gentle is expected for "masu/desu/kudasai"
    assert!(matches!(profile.tags.tone_hint, ToneHint::Gentle));

    // Direct
    let id2 = core.start_message();
    let text_direct = "これをやれ。";
    let text_direct_2 = "これは重要だ。";
    for ch in text_direct_2.chars() {
        core.push_event(&id2, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id2, InputEvent::Submit { ts }).unwrap();
    let json2 = core.finalize_message(&id2, text_direct_2).unwrap();
    let profile2: ifl_core::InputProfile = serde_json::from_str(&json2).unwrap();

    assert!(matches!(profile2.tags.tone_hint, ToneHint::Direct));
}

#[test]
fn test_persistence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello"
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Export
    let events_json = core.export_events(&id).unwrap();
    println!("Exported events: {}", events_json);

    // Import into new core
    let core2 = IflCore::new();
    let id2 = core2.import_events(&events_json).unwrap();

    // Finalize imported session
    let json = core2.finalize_message(&id2, "Hello").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(profile.source.source_type, SourceType::TypedOnly);
    assert_eq!(profile.structure.char_count, 5);
}

#[test]
fn test_confidence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Explicit request "Summarize this"
    let text = "Summarize this article.";
    for ch in text.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, text).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // Should have high confidence due to explicit request
    assert!(profile.tags.confidence > 0.7);
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Summarize));
}

#[test]
fn test_efficiency_score() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello" (5 chars)
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Backspace 2 chars
    for _ in 0..2 {
        core.push_event(
            &id,
            InputEvent::KeyDelete {
                kind: ifl_core::event::DeleteKind::Backspace,
                count: 1,
                ts,
            },
        )
        .unwrap();
        ts += 100;
    }

    // Type "p!" (2 chars) -> "Help!"
    for ch in "p!".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Final text "Help!" (5 chars)
    // Total typed: 5 (Hello) + 2 (p!) = 7 chars
    // Efficiency = 5 / 7 = ~0.71

    let json = core.finalize_message(&id, "Help!").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();
    println!("Efficiency: {}", profile.editing.efficiency_score);
    assert!(profile.editing.efficiency_score > 0.7 && profile.editing.efficiency_score < 0.72);
}

#[test]
fn test_snapshot_persistence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Snap"
    for ch in "Snap".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Export snapshot
    println!("Exporting snapshot...");
    let snapshot_json = core.export_snapshot(&id, "Snap").unwrap();
    println!("Snapshot JSON len: {}", snapshot_json.len());

    let snapshot: ifl_core::profile::SessionSnapshot =
        serde_json::from_str(&snapshot_json).unwrap();

    // Verify profile
    assert_eq!(snapshot.profile.structure.char_count, 4);
    assert_eq!(snapshot.profile.source.source_type, SourceType::TypedOnly);

    // Verify events
    println!("Snapshot events len: {}", snapshot.events.len());
    if !snapshot.events.is_empty() {
        println!("First event: {:?}", snapshot.events[0]);
    }
    assert!(
        snapshot.events.len() >= 5,
        "Expected >= 5 events, got {}",
        snapshot.events.len()
    ); // 4 chars + submit
    assert!(matches!(
        snapshot.events[0],
        InputEvent::KeyInsert { ch: 'S', .. }
    ));
}

#[test]
fn test_scenario_japanese_tone() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Polite
    let text_polite = "お願いします。";
    for ch in text_polite.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();
    let json = core.finalize_message(&id, text_polite).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // ToneHint::Gentle is expected for "masu/desu/kudasai"
    assert!(matches!(profile.tags.tone_hint, ToneHint::Gentle));

    // Direct
    let id2 = core.start_message();
    let text_direct = "これをやれ。";
    let text_direct_2 = "これは重要だ。";
    for ch in text_direct_2.chars() {
        core.push_event(&id2, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id2, InputEvent::Submit { ts }).unwrap();
    let json2 = core.finalize_message(&id2, text_direct_2).unwrap();
    let profile2: ifl_core::InputProfile = serde_json::from_str(&json2).unwrap();

    assert!(matches!(profile2.tags.tone_hint, ToneHint::Direct));
}

#[test]
fn test_persistence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello"
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Export
    let events_json = core.export_events(&id).unwrap();
    println!("Exported events: {}", events_json);

    // Import into new core
    let core2 = IflCore::new();
    let id2 = core2.import_events(&events_json).unwrap();

    // Finalize imported session
    let json = core2.finalize_message(&id2, "Hello").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(profile.source.source_type, SourceType::TypedOnly);
    assert_eq!(profile.structure.char_count, 5);
}

#[test]
fn test_confidence() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Explicit request "Summarize this"
    let text = "Summarize this article.";
    for ch in text.chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    let json = core.finalize_message(&id, text).unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    // Should have high confidence due to explicit request
    assert!(profile.tags.confidence > 0.7);
    assert!(profile.tags.answer_mode.contains(&AnswerMode::Summarize));
}

#[test]
fn test_efficiency_score() {
    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000;

    // Type "Hello" (5 chars)
    for ch in "Hello".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }

    // Backspace 2 chars
    for _ in 0..2 {
        core.push_event(
            &id,
            InputEvent::KeyDelete {
                kind: ifl_core::event::DeleteKind::Backspace,
                count: 1,
                ts,
            },
        )
        .unwrap();
        ts += 100;
    }

    // Type "p!" (2 chars) -> "Help!"
    for ch in "p!".chars() {
        core.push_event(&id, InputEvent::KeyInsert { ch, ts })
            .unwrap();
        ts += 100;
    }
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Final text "Help!" (5 chars)
    // Total typed: 5 (Hello) + 2 (p!) = 7 chars
    // Efficiency = 5 / 7 = ~0.71

    let json = core.finalize_message(&id, "Help!").unwrap();
    let profile: ifl_core::InputProfile = serde_json::from_str(&json).unwrap();

    println!("Efficiency: {}", profile.editing.efficiency_score);
    assert!(profile.editing.efficiency_score > 0.7 && profile.editing.efficiency_score < 0.72);
}
