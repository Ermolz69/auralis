use super::*;
use domain::speech::SpeechId;
use ports::cancellation::CancellationToken;
use std::{
    io::{self, Cursor},
    time::Duration,
};

struct CancellingInput {
    stream: Cursor<Vec<u8>>,
    cancel: CancellationToken,
}

impl Read for CancellingInput {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let read = self.stream.read(bytes)?;
        self.cancel.cancel();
        Ok(read)
    }
}

impl Seek for CancellingInput {
    fn seek(&mut self, offset: SeekFrom) -> io::Result<u64> {
        self.stream.seek(offset)
    }
}

#[test]
fn cancellation_after_first_pcm_block_prevents_complete_extraction()
-> Result<(), Box<dyn std::error::Error>> {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/revoice/bridge-unit");
    std::fs::create_dir_all(&parent)?;
    let directory = tempfile::tempdir_in(parent)?;
    let cancel = CancellationToken::new();
    let mut input = CancellingInput {
        stream: Cursor::new(vec![0; 262144]),
        cancel: cancel.clone(),
    };
    let properties = WavProperties {
        sample_rate: 48000,
        channels: 2,
        frames: 65536,
        duration_ms: 1365,
        data_offset: 0,
        data_bytes: 262144,
    };
    let range = UtteranceAudioRange {
        utterance_id: SpeechId::parse("one")?,
        first_frame: 0,
        frame_count: 65536,
        chunk_ids: vec!["one-chunk-0001".into()],
        source_window: None,
    };
    let path = directory.path().join("partial.wav");
    let result = extract(
        &mut input,
        &properties,
        &range,
        &path,
        &WorkControl::new(cancel, Duration::from_secs(2))?,
    );
    assert!(matches!(result, Err(PortError::Cancelled)));
    assert_eq!(std::fs::metadata(path)?.len(), 44 + 65536);
    Ok(())
}
