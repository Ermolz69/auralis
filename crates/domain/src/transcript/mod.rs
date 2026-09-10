use uuid::Uuid;

use crate::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TranscriptSegmentId(pub Uuid);

impl TranscriptSegmentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TranscriptSegmentId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TranscriptSegment {
    pub id: TranscriptSegmentId,
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_text: String,
    pub translated_text: Option<String>,
    pub adapted_text: Option<String>,
    pub speaker: Option<String>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transcript {
    pub language: String,
    pub segments: Vec<TranscriptSegment>,
}

impl Transcript {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.language.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Transcript language cannot be empty".to_string(),
            ));
        }

        let mut ids = std::collections::HashSet::with_capacity(self.segments.len());
        let mut indexes = std::collections::HashSet::with_capacity(self.segments.len());
        for segment in &self.segments {
            if segment.end_ms < segment.start_ms {
                return Err(DomainError::ValidationError(
                    "Transcript segment end cannot precede its start".to_string(),
                ));
            }
            if segment.source_text.trim().is_empty() {
                return Err(DomainError::ValidationError(
                    "Transcript segment text cannot be empty".to_string(),
                ));
            }
            if !ids.insert(&segment.id) || !indexes.insert(segment.index) {
                return Err(DomainError::ValidationError(
                    "Transcript segment IDs and indexes must be unique".to_string(),
                ));
            }
            if segment
                .confidence
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            {
                return Err(DomainError::ValidationError(
                    "Transcript segment confidence must be between 0 and 1".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Transcript, TranscriptSegment, TranscriptSegmentId};

    fn segment() -> TranscriptSegment {
        TranscriptSegment {
            id: TranscriptSegmentId::new(),
            index: 0,
            start_ms: 10,
            end_ms: 20,
            source_text: "valid".to_string(),
            translated_text: None,
            adapted_text: None,
            speaker: None,
            confidence: Some(0.9),
        }
    }

    #[test]
    fn rejects_invalid_segment_timing_and_confidence() {
        let mut transcript = Transcript {
            language: "en".to_string(),
            segments: vec![segment()],
        };
        transcript.segments[0].end_ms = 9;
        assert!(transcript.validate().is_err());

        transcript.segments[0].end_ms = 20;
        transcript.segments[0].confidence = Some(f32::NAN);
        assert!(transcript.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_ids_and_indexes() {
        let first = segment();
        let mut duplicate = segment();
        duplicate.id = first.id.clone();
        duplicate.index = first.index;
        let transcript = Transcript {
            language: "en".to_string(),
            segments: vec![first, duplicate],
        };

        assert!(transcript.validate().is_err());
    }
}
