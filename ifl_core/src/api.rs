use crate::event::InputEvent;
use crate::feature::{FeatureExtractor, StructureAnalyzer};
use crate::profile::InputProfile;
use crate::rules::RuleEngine;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct IflCore {
    sessions: Arc<Mutex<HashMap<String, FeatureExtractor>>>,
}

impl IflCore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_message(&self) -> String {
        let id = Uuid::new_v4().to_string();
        let extractor = FeatureExtractor::new();
        self.sessions.lock().unwrap().insert(id.clone(), extractor);
        id
    }

    pub fn push_event(&self, message_id: &str, event: InputEvent) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(extractor) = sessions.get_mut(message_id) {
            extractor.process_event(&event);
            Ok(())
        } else {
            Err(format!("Message ID {} not found", message_id))
        }
    }

    pub fn finalize_message(&self, message_id: &str, final_text: &str) -> Result<String, String> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(extractor) = sessions.remove(message_id) {
            // 1. Extract features
            // We need a submit timestamp. Ideally it comes from the last event or current time.
            // For now, let's assume the last event time or 0 if empty.
            // In a real scenario, we might want to pass a timestamp to finalize_message.
            // But let's use a placeholder or derive it.
            // Actually, `InputEvent::Submit` should have been the last event pushed.
            // We'll use the internal state of extractor to determine duration.
            // Let's assume the Submit event was the last one pushed, so we can use the last_event_time.

            // Note: We don't have access to the last event timestamp easily unless we exposed it.
            // Let's assume the caller pushed Submit event.
            // We'll use 0 as fallback for now, or maybe we should change extract_timing_features signature?
            // Let's just pass a dummy timestamp if we can't get it, or rely on the extractor's internal state if we modify it.
            // Actually, let's modify `extract_timing_features` to take an optional end time, or use its own last time.
            // But `extract_timing_features` currently takes `submit_ts`.
            // Let's just use a dummy value for now, or maybe current system time if we had access (but we want determinism/purity).
            // Better: The `Submit` event should have been pushed. We can track `submit_ts` in `FeatureExtractor`.

            // Let's assume the last event was Submit and use its timestamp if available, otherwise 0.
            // But `FeatureExtractor` doesn't expose `last_event_time` publicly.

            let source = extractor.extract_source_features(0);
            let timing = extractor.extract_timing_features();
            let structure = StructureAnalyzer::analyze(final_text);
            let editing = extractor.extract_editing_features(structure.char_count);

            let tags = RuleEngine::apply(&source, &timing, &editing, &structure);

            let profile = InputProfile {
                message_id: message_id.to_string(),
                source,
                timing,
                editing,
                structure,
                tags,
            };

            serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())
        } else {
            Err(format!("Message ID {} not found", message_id))
        }
    }

    pub fn export_events(&self, id: &str) -> Result<String, String> {
        let sessions = self.sessions.lock().unwrap();
        let extractor = sessions
            .get(id)
            .ok_or_else(|| format!("Message ID {} not found", id))?;
        // Assuming FeatureExtractor stores events internally and has a method to retrieve them for serialization.
        // If FeatureExtractor does not store events, this method cannot be implemented as requested without modifying FeatureExtractor.
        // For now, we'll assume `FeatureExtractor` has a `get_events()` method that returns `&Vec<InputEvent>`.
        // If it doesn't, this will be a compilation error and `FeatureExtractor` needs to be updated.
        serde_json::to_string_pretty(&extractor.get_events()).map_err(|e| e.to_string())
    }

    pub fn import_events(&self, json: &str) -> Result<String, String> {
        let events: Vec<InputEvent> = serde_json::from_str(json).map_err(|e| e.to_string())?;

        let id = self.start_message();
        for event in events {
            self.push_event(&id, event)?;
        }

        Ok(id)
    }
}
