use domain::media::{MediaMetadata, MediaSource};

#[derive(Debug, Clone)]
pub enum ProjectUpdate {
    Rename {
        title: String,
    },
    ImportSource {
        source: MediaSource,
        metadata: Option<Box<MediaMetadata>>,
    },
    MarkReadyForProcessing,
}
