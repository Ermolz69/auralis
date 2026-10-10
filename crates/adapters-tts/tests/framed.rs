use adapters_tts::WorkerFrameReader;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn reads_complete_frames_across_chunk_boundaries() {
    let (mut writer, reader) = tokio::io::duplex(32);
    let sender = tokio::spawn(async move {
        assert!(writer.write_all(b"{\"event\":").await.is_ok());
        assert!(writer.write_all(b"1}\n{\"event\":2}\n").await.is_ok());
    });
    let mut framed = WorkerFrameReader::new(reader);
    assert!(matches!(framed.next_frame().await, Ok(Some(value)) if value == b"{\"event\":1}\n"));
    assert!(matches!(framed.next_frame().await, Ok(Some(value)) if value == b"{\"event\":2}\n"));
    assert!(matches!(framed.next_frame().await, Ok(None)));
    assert!(sender.await.is_ok());
}

#[tokio::test]
async fn rejects_partial_eof_and_oversized_frames() {
    let (mut writer, reader) = tokio::io::duplex(32);
    assert!(writer.write_all(b"incomplete").await.is_ok());
    drop(writer);
    let mut framed = WorkerFrameReader::new(reader);
    assert!(framed.next_frame().await.is_err());

    let (mut writer, reader) = tokio::io::duplex(8192);
    let sender = tokio::spawn(async move {
        let excess = vec![b'x'; 1_048_577];
        let _ = writer.write_all(&excess).await;
    });
    let mut framed = WorkerFrameReader::new(reader);
    assert!(framed.next_frame().await.is_err());
    drop(framed);
    let _ = sender.await;
}
