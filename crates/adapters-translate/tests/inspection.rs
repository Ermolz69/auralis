use adapters_translate::{FormatInspector, TranslateDb};
use auralis_translation::SourceHash;
use auralis_translation_sqlite::SqliteConfig;
use ports::translation_inspection::{TranslationInspectorPort, TranslationSourceFormat};
use std::error::Error;

#[tokio::test]
async fn strict_inspection_preserves_original_and_opens_second_database()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let srt_path = directory.path().join("source.srt");
    let vtt_path = directory.path().join("source.vtt");
    let srt = b"1\r\n00:00:01,000 --> 00:00:02,000\r\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\r\n";
    let vtt = b"WEBVTT\r\n\r\nNOTE provenance\r\nsynthetic\r\n\r\n00:01.000 --> 00:02.000\r\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\r\n";
    std::fs::write(&srt_path, srt)?;
    std::fs::write(&vtt_path, vtt)?;
    let inspector = FormatInspector;

    let srt_report = inspector
        .inspect(&srt_path, TranslationSourceFormat::Srt)
        .await?;
    assert_eq!(
        srt_report.source_sha256,
        SourceHash::digest(srt).to_string()
    );
    assert_eq!(srt_report.segments.len(), 1);
    assert_eq!(srt_report.segments[0].cue_label.as_deref(), Some("1"));
    assert_eq!(srt_report.segments[0].source_lines, ["你好。"]);
    let vtt_report = inspector
        .inspect(&vtt_path, TranslationSourceFormat::Vtt)
        .await?;
    assert_eq!(
        vtt_report.source_sha256,
        SourceHash::digest(vtt).to_string()
    );
    assert_eq!(vtt_report.segments.len(), 1);
    assert_eq!(vtt_report.segments[0].cue_label, None);
    assert_eq!(vtt_report.segments[0].source_lines, ["你好。"]);
    assert_eq!(std::fs::read(&srt_path)?, srt);
    assert_eq!(std::fs::read(&vtt_path)?, vtt);

    let db = TranslateDb::open(
        &directory.path().join("auralis-translate.sqlite"),
        SqliteConfig::default(),
    )?;
    assert_eq!(db.schema_version()?, 5);
    Ok(())
}

#[tokio::test]
async fn unsupported_webvtt_is_rejected_without_creating_translation_state()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source_path = directory.path().join("unsupported.vtt");
    let source = b"WEBVTT\n\nSTYLE\n::cue { color: red }\n\n00:01.000 --> 00:02.000\nhello\n";
    std::fs::write(&source_path, source)?;
    let result = FormatInspector
        .inspect(&source_path, TranslationSourceFormat::Vtt)
        .await;
    assert!(result.is_err());
    assert_eq!(std::fs::read(&source_path)?, source);
    assert!(!directory.path().join("auralis-translate.sqlite").exists());
    Ok(())
}
