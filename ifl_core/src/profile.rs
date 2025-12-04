use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputProfile {
    pub message_id: String,
    pub source: SourceFeatures,
    pub timing: TimingFeatures,
    pub editing: EditingFeatures,
    pub structure: StructureFeatures,
    pub tags: AnswerTags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFeatures {
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub paste_ratio: f32,
    pub paste_events: usize,
    pub first_action: FirstAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    TypedOnly,
    PasteOnly,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstAction {
    Paste,
    Typed,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingFeatures {
    pub total_duration_ms: u64,
    pub avg_chars_per_sec: f32,
    pub typing_bursts: usize,
    pub long_pause_count: usize,
    pub pre_submit_pause_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditingFeatures {
    pub backspace_count: usize,
    pub backspace_burst_count: usize,
    pub undo_count: usize,
    pub redo_count: usize,
    pub selection_edit_count: usize,
    pub efficiency_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureFeatures {
    pub char_count: usize,
    pub line_count: usize,
    pub avg_line_length: f32,
    pub bullet_lines: usize,
    pub has_code_block: bool,
    pub question_like: bool,
    pub command_like: bool,
    pub japanese_detected: bool,
    pub request_summary: bool,
    pub request_implementation: bool,
    pub is_polite: bool,
    pub is_direct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerTags {
    pub answer_mode: Vec<AnswerMode>,
    pub scope_hint: ScopeHint,
    pub tone_hint: ToneHint,

    pub depth_hint: DepthHint,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnswerMode {
    Summarize,
    Structure,
    Refine,
    Explore,
    Complete,
    ClarifyQuestion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeHint {
    Narrow,
    Medium,
    Broad,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToneHint {
    Direct,
    Gentle,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DepthHint {
    Shallow,
    Normal,
    Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub profile: InputProfile,
    pub events: Vec<crate::event::InputEvent>,
}
