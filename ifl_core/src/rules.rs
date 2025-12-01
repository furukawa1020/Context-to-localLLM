use crate::profile::{
    AnswerMode, AnswerTags, DepthHint, EditingFeatures, ScopeHint, SourceFeatures, SourceType,
    StructureFeatures, TimingFeatures, ToneHint,
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

        // Rule 1: High paste ratio + multiple lines -> Summarize/Structure
        if source.paste_ratio > 0.8 && structure.line_count >= 3 {
            modes.insert(AnswerMode::Summarize);
            modes.insert(AnswerMode::Structure);
            scope = ScopeHint::Broad;
        }

        // Rule 2: Long typed session with edits -> Refine/Clarify
        if matches!(source.source_type, SourceType::TypedOnly)
            && timing.total_duration_ms > 30_000
            && editing.backspace_count > 20
        {
            modes.insert(AnswerMode::Refine);
            modes.insert(AnswerMode::ClarifyQuestion);
            depth = DepthHint::Deep;
        }

        // Rule 3: Short query -> Explore/Clarify
        if structure.line_count <= 2 && structure.char_count < 40 {
            modes.insert(AnswerMode::Explore);
            modes.insert(AnswerMode::ClarifyQuestion);
            scope = ScopeHint::Broad;
        }

        // Rule 4: Mixed source with selection edits -> Complete
        if matches!(source.source_type, SourceType::Mixed) && editing.selection_edit_count > 2 {
            modes.insert(AnswerMode::Complete);
        }

        // Rule 5: Bullet points -> Structure
        if structure.bullet_lines > 2 {
            modes.insert(AnswerMode::Structure);
            scope = ScopeHint::Narrow;
        }

        // Rule 6: Question like -> Clarify/Explore
        if structure.question_like {
            modes.insert(AnswerMode::ClarifyQuestion);
        }

        // Rule 7: Command like -> Direct tone
        if structure.command_like {
            tone = ToneHint::Direct;
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
        }
        }

        // Rule 9: Explicit requests
        if structure.request_summary {
            modes.insert(AnswerMode::Summarize);
            scope = ScopeHint::Broad;
        }
        if structure.request_implementation {
            modes.insert(AnswerMode::Complete);
            modes.insert(AnswerMode::Structure);
            tone = ToneHint::Direct;
        }

        // Fallback if no modes
        if modes.is_empty() {
            modes.insert(AnswerMode::Explore);
        }

        // Convert HashSet to Vec
        let mut answer_mode: Vec<AnswerMode> = modes.into_iter().collect();
        // Sort for deterministic output (optional but good for testing)
        // answer_mode.sort(); // Need Ord derived or manual sort, skipping for now as enum doesn't derive Ord by default

        AnswerTags {
            answer_mode,
            scope_hint: scope,
            tone_hint: tone,
            depth_hint: depth,
        }
    }
}
