use crate::profile::{
    AnswerMode, AnswerTags, DepthHint, EditingFeatures, ScopeHint, SourceFeatures, SourceType,
    StructureFeatures, TimingFeatures, ToneHint, UserState,
};
use std::collections::HashSet;

pub struct RuleEngine;

impl RuleEngine {
    pub fn apply(
        source: &SourceFeatures,
        timing: &TimingFeatures,
        editing: &EditingFeatures,
        structure: &StructureFeatures,
    ) -> AnswerTags {
        let mut modes = HashSet::new();
        let mut scope = ScopeHint::Medium; // Default
        let mut tone = ToneHint::Neutral; // Default
        let mut depth = DepthHint::Normal; // Default
        let mut confidence = 0.5f32; // Base confidence

        // Rule 1: High paste ratio + multiple lines -> Summarize/Structure
        if source.paste_ratio > 0.8 && structure.line_count >= 3 {
            modes.insert(AnswerMode::Summarize);
            modes.insert(AnswerMode::Structure);
            scope = ScopeHint::Broad;
            confidence += 0.2;
        }

        // Rule 2: Long typed session with edits -> Refine/Clarify
        if matches!(source.source_type, SourceType::TypedOnly)
            && timing.total_duration_ms > 30_000
            && editing.backspace_count > 20
        {
            modes.insert(AnswerMode::Refine);
            modes.insert(AnswerMode::ClarifyQuestion);
            depth = DepthHint::Deep;
            confidence += 0.2;
        }

        // Rule 3: Short query -> Explore/Clarify
        if structure.line_count <= 2 && structure.char_count < 40 {
            modes.insert(AnswerMode::Explore);
            modes.insert(AnswerMode::ClarifyQuestion);
            scope = ScopeHint::Broad;
            confidence += 0.1;
        }

        // Rule 4: Mixed source with selection edits -> Complete
        if matches!(source.source_type, SourceType::Mixed) && editing.selection_edit_count > 2 {
            modes.insert(AnswerMode::Complete);
            confidence += 0.2;
        }

        // Rule 5: Bullet points -> Structure
        if structure.bullet_lines > 2 {
            modes.insert(AnswerMode::Structure);
            scope = ScopeHint::Narrow;
            confidence += 0.1;
        }

        // Rule 6: Question like -> Clarify/Explore
        if structure.question_like {
            modes.insert(AnswerMode::ClarifyQuestion);
            confidence += 0.1;
        }

        // Rule 7: Command like -> Direct tone
        if structure.command_like {
            tone = ToneHint::Direct;
            confidence += 0.1;
        }

        // Rule 8: Japanese specific rules
        if structure.japanese_detected {
            // Japanese text tends to be denser, so "Short" threshold might be lower
            if structure.char_count > 500 {
                depth = DepthHint::Deep;
            }
            // Japanese Tone Detection
            if structure.is_polite {
                tone = ToneHint::Gentle;
            } else if structure.is_direct {
                tone = ToneHint::Direct;
            }
            confidence += 0.1;
        }

        // Rule 9: Explicit requests
        if structure.request_summary {
            modes.insert(AnswerMode::Summarize);
            scope = ScopeHint::Broad;
            confidence += 0.3; // Explicit request is strong
        }
        if structure.request_implementation {
            modes.insert(AnswerMode::Complete);
            modes.insert(AnswerMode::Structure);
            tone = ToneHint::Direct;
            confidence += 0.3; // Explicit request is strong
        }

        // Fallback if no modes
        if modes.is_empty() {
            modes.insert(AnswerMode::Explore);
        }

        // Convert HashSet to Vec
        let mut answer_mode: Vec<AnswerMode> = modes.into_iter().collect();
        // Sort for deterministic output (optional but good for testing)
        // answer_mode.sort(); // Need Ord derived or manual sort, skipping for now as enum doesn't derive Ord by default

        // User State Detection
        let mut user_states = HashSet::new();

        // Hesitant: Low speed + many pauses
        if timing.avg_chars_per_sec < 2.0 && timing.long_pause_count > 2 {
            user_states.insert(UserState::Hesitant);
        }

        // Flowing: High speed + few pauses
        if timing.avg_chars_per_sec > 5.0 && timing.long_pause_count == 0 {
            user_states.insert(UserState::Flowing);
        }

        // Editing: High backspace count
        if editing.backspace_count > 10 || editing.selection_edit_count > 2 {
            user_states.insert(UserState::Editing);
        }

        // Pasting: High paste ratio
        if source.paste_ratio > 0.5 {
            user_states.insert(UserState::Pasting);
        }

        // Scattered: Many bursts + short segments (heuristic)
        if timing.typing_bursts > 5 && timing.avg_chars_per_sec < 3.0 {
            user_states.insert(UserState::Scattered);
        }

        // Focused: High speed + few edits
        if timing.avg_chars_per_sec > 4.0 && editing.backspace_count < 5 {
            user_states.insert(UserState::Focused);
        }

        let user_state: Vec<UserState> = user_states.into_iter().collect();

        AnswerTags {
            answer_mode,
            scope_hint: scope,
            tone_hint: tone,
            depth_hint: depth,
            user_state,
            confidence: confidence.min(1.0),
        }
    }
}
