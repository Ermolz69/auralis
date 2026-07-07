use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub language: String,
    pub segments: Vec<TranscriptSegment>,
}
