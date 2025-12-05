use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum InputEvent {
    KeyInsert {
        ch: char,
        ts: u64,
    },
    KeyDelete {
        kind: DeleteKind,
        count: u32,
        ts: u64,
    },
    Paste {
        length: usize,
        ts: u64,
    },
    Cut {
        length: usize,
        ts: u64,
    },
    CursorMove {
        position: usize,
        ts: u64,
    },
    SelectionChange {
        start: usize,
        end: usize,
        ts: u64,
    },
    CompositionStart {
        ts: u64,
    },
    CompositionEnd {
        ts: u64,
    },
    Submit {
        ts: u64,
    },
    Undo {
        ts: u64,
    },
    Redo {
        ts: u64,
    },
    GhostText {
        text: String,
        ts: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeleteKind {
    Backspace,
    Delete,
}
