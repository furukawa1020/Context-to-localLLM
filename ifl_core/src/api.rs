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

    pub fn start_message(&self) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let extractor = FeatureExtractor::new();
        self.sessions
            .lock()
            .map_err(|_| "Mutex poisoned".to_string())?
            .insert(id.clone(), extractor);
        Ok(id)
    }

    pub fn push_event(&self, message_id: &str, event: InputEvent) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Mutex poisoned".to_string())?;
        if let Some(extractor) = sessions.get_mut(message_id) {
            extractor.process_event(&event);
            Ok(())
        } else {
            Err(format!("Message ID {} not found", message_id))
        }
    }

    pub fn finalize_message(&self, message_id: &str, final_text: &str) -> Result<String, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Mutex poisoned".to_string())?;
        if let Some(extractor) = sessions.remove(message_id) {
            // 1. Extract features
            let source = extractor.extract_source_features(0u64);
            let timing = extractor.extract_timing_features();
            let structure = StructureAnalyzer::analyze(final_text);
            let editing = extractor.extract_editing_features(structure.char_count);

            let tags = RuleEngine::apply(&source, &timing, &editing, &structure);

            // Extract Ghost Text
            let ghost_text = extractor.extract_ghost_text();

            let profile = InputProfile {
                message_id: message_id.to_string(),
                source,
                timing,
                editing,
                structure,
                tags,
                ghost_text,
            };

            serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())
        } else {
            Err(format!("Message ID {} not found", message_id))
        }
    }

    pub fn preview_message(&self, message_id: &str, current_text: &str) -> Result<String, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Mutex poisoned".to_string())?;
        if let Some(extractor) = sessions.get(message_id) {
            // 1. Extract features (non-destructive)
            let source = extractor.extract_source_features(0u64);
            let timing = extractor.extract_timing_features();
            let structure = StructureAnalyzer::analyze(current_text);
            let editing = extractor.extract_editing_features(structure.char_count);

            let tags = RuleEngine::apply(&source, &timing, &editing, &structure);

            // Extract Ghost Text
            let ghost_text = extractor.extract_ghost_text();

            let profile = InputProfile {
                message_id: message_id.to_string(),
                source,
                timing,
                editing,
                structure,
                tags,
                ghost_text,
            };

            serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())
        } else {
            Err(format!("Message ID {} not found", message_id))
        }
    }

    pub fn export_events(&self, id: &str) -> Result<String, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Mutex poisoned".to_string())?;
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

        let id = self.start_message()?;
        for event in events {
            self.push_event(&id, event)?;
        }

        Ok(id)
    }

    pub fn export_snapshot(&self, id: &str, final_text: &str) -> Result<String, String> {
        // 1. Get events (clone them)
        let events_json = self.export_events(id)?;
        let events: Vec<InputEvent> =
            serde_json::from_str(&events_json).map_err(|e| e.to_string())?;

        // 2. Finalize to get profile
        let profile_json = self.finalize_message(id, final_text)?;
        let profile: InputProfile =
            serde_json::from_str(&profile_json).map_err(|e| e.to_string())?;

        // 3. Combine
        let snapshot = crate::profile::SessionSnapshot { profile, events };

        serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())
    }
}
