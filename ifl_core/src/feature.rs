use crate::event::{DeleteKind, InputEvent};
use crate::profile::{
    EditingFeatures, FirstAction, SourceFeatures, SourceType, StructureFeatures, TimingFeatures,
};

pub struct FeatureExtractor {
    // State
    start_time: Option<u64>,
    last_event_time: Option<u64>,
    
    // Source stats
    first_action: Option<FirstAction>,
    paste_events: usize,
    total_pasted_chars: usize,
    total_typed_chars: usize, // For calculating paste ratio
    
    // Timing stats
    typing_bursts: usize,
    long_pause_count: usize,
    
    // Editing stats
    backspace_count: usize,
    backspace_burst_count: usize,
    undo_count: usize,
    redo_count: usize,
    selection_edit_count: usize,
    
    // Internal tracking
    in_backspace_burst: bool,
    paste_timestamps: Vec<u64>, // To check beginning/end
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self {
            start_time: None,
            last_event_time: None,
            first_action: None,
            paste_events: 0,
            total_pasted_chars: 0,
            total_typed_chars: 0,
            typing_bursts: 0,
            long_pause_count: 0,
            backspace_count: 0,
            backspace_burst_count: 0,
            undo_count: 0,
            redo_count: 0,
            selection_edit_count: 0,
            in_backspace_burst: false,
            paste_timestamps: Vec::new(),
        }
    }

    pub fn process_event(&mut self, event: &InputEvent) {
        let ts = match event {
            InputEvent::KeyInsert { ts, .. } => *ts,
            InputEvent::KeyDelete { ts, .. } => *ts,
            InputEvent::Paste { ts, .. } => *ts,
            InputEvent::Cut { ts, .. } => *ts,
            InputEvent::CursorMove { ts, .. } => *ts,
            InputEvent::SelectionChange { ts, .. } => *ts,
            InputEvent::CompositionStart { ts } => *ts,
            InputEvent::CompositionEnd { ts } => *ts,
            InputEvent::Submit { ts } => *ts,
            InputEvent::Undo { ts } => *ts,
            InputEvent::Redo { ts } => *ts,
        };

        if self.start_time.is_none() {
            self.start_time = Some(ts);
        }

        // First action detection
        if self.first_action.is_none() {
            match event {
                InputEvent::Paste { .. } => self.first_action = Some(FirstAction::Paste),
                InputEvent::KeyInsert { .. } => self.first_action = Some(FirstAction::Typed),
                _ => {} // Wait for first significant action
            }
        }

        // Timing analysis
        if let Some(last_ts) = self.last_event_time {
            let diff = ts.saturating_sub(last_ts);
            if diff > 1500 {
                self.long_pause_count += 1;
                // End of a burst
                self.typing_bursts += 1;
            }
        } else {
            // First event starts a burst
            self.typing_bursts += 1;
        }
        self.last_event_time = Some(ts);

        // Event specific logic
        match event {
            InputEvent::KeyInsert { .. } => {
                self.total_typed_chars += 1;
                self.in_backspace_burst = false;
            }
            InputEvent::KeyDelete { kind, count, .. } => {
                if matches!(kind, DeleteKind::Backspace) {
                    self.backspace_count += *count as usize;
                    if self.in_backspace_burst {
                        // Continue burst
                    } else {
                        self.backspace_burst_count += 1;
                        self.in_backspace_burst = true;
                    }
                } else {
                    self.in_backspace_burst = false;
                }
            }
            InputEvent::Paste { length, .. } => {
                self.paste_events += 1;
                self.total_pasted_chars += *length;
                self.paste_timestamps.push(ts);
                self.in_backspace_burst = false;
            }
            InputEvent::Undo { .. } => {
                self.undo_count += 1;
                self.in_backspace_burst = false;
            }
            InputEvent::Redo { .. } => {
                self.redo_count += 1;
                self.in_backspace_burst = false;
            }
            InputEvent::SelectionChange { .. } => {
                // Logic for selection_edit_count would require state of previous selection
                // Simplified: if we get a KeyInsert or Paste immediately after SelectionChange with range > 0
                // For now, we'll leave this as a placeholder or need more complex state tracking
                self.in_backspace_burst = false;
            }
            _ => {
                self.in_backspace_burst = false;
            }
        }
    }

    pub fn extract_source_features(&self, total_duration: u64) -> SourceFeatures {
        let total_chars = self.total_typed_chars + self.total_pasted_chars;
        let paste_ratio = if total_chars > 0 {
            self.total_pasted_chars as f32 / total_chars as f32
        } else {
            0.0
        };

        let source_type = if self.total_typed_chars == 0 && self.total_pasted_chars > 0 {
            SourceType::PasteOnly
        } else if self.total_pasted_chars == 0 && self.total_typed_chars > 0 {
            SourceType::TypedOnly
        } else {
            SourceType::Mixed
        };

        SourceFeatures {
            source_type,
            paste_ratio,
            paste_events: self.paste_events,
            first_action: self.first_action.clone().unwrap_or(FirstAction::Other),
        }
    }

    pub fn extract_timing_features(&self) -> TimingFeatures {
        let last_ts = self.last_event_time.unwrap_or(0);
        let start = self.start_time.unwrap_or(last_ts);
        let total_duration_ms = last_ts.saturating_sub(start);
        
        let avg_chars_per_sec = if total_duration_ms > 0 {
            (self.total_typed_chars as f32 / (total_duration_ms as f32 / 1000.0))
        } else {
            0.0
        };

        let pre_submit_pause_ms = 0; // Simplified: last event IS submit usually, so pause is 0 unless we track previous to last.
        // If we want pre-submit pause, we need to track the event BEFORE submit.
        // For now, let's leave it as 0 or implement better tracking if needed.
        // Actually, if last_event_time is Submit, we need the time of the event before that.
        // But let's stick to simple fix first: get duration working.

        TimingFeatures {
            total_duration_ms,
            avg_chars_per_sec,
            typing_bursts: self.typing_bursts,
            long_pause_count: self.long_pause_count,
            pre_submit_pause_ms,
        }
    }

    pub fn extract_editing_features(&self) -> EditingFeatures {
        EditingFeatures {
            backspace_count: self.backspace_count,
            backspace_burst_count: self.backspace_burst_count,
            undo_count: self.undo_count,
            redo_count: self.redo_count,
            selection_edit_count: self.selection_edit_count,
        }
    }
}

pub struct StructureAnalyzer;

impl StructureAnalyzer {
    pub fn analyze(text: &str) -> StructureFeatures {
        let char_count = text.chars().count();
        let lines: Vec<&str> = text.lines().collect();
        let line_count = lines.len();
        
        let avg_line_length = if line_count > 0 {
            char_count as f32 / line_count as f32
        } else {
            0.0
        };

        let bullet_lines = lines.iter().filter(|l| {
            let trimmed = l.trim_start();
            trimmed.starts_with("- ") || trimmed.starts_with("* ") || 
            (trimmed.chars().next().map_or(false, |c| c.is_digit(10)) && trimmed.contains(". "))
        }).count();

        let has_code_block = text.contains("```") || lines.iter().any(|l| l.starts_with("    ") || l.starts_with("\t"));

        let question_like = text.trim().ends_with('?') || text.contains('?');
        
        let command_like = {
            let lower = text.to_lowercase();
            lower.starts_with("please") || lower.starts_with("write") || lower.starts_with("create")
        };

        StructureFeatures {
            char_count,
            line_count,
            avg_line_length,
            bullet_lines,
            has_code_block,
            question_like,
            command_like,
        }
    }
}
