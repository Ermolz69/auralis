#![cfg(windows)]
use adapters_ytdlp::ytdlp::YtDlpAdapter;
use domain::media::{ArtifactLocation, MediaSource, SubtitleTrack};
use ports::{
    cancellation::CancellationToken,
    error::PortError,
    source::{DownloadSubtitleRequest, SubtitleSourcePort},
};

fn request(target: std::path::PathBuf, format: &str) -> DownloadSubtitleRequest {
    DownloadSubtitleRequest {
        source: MediaSource::YoutubeUrl {
            url: "https://www.youtube.com/watch?v=fixture".into(),
        },
        track: SubtitleTrack {
            id: format!("zh-manual-{format}"),
            language: "zh".into(),
            label: None,
            format: Some(format.into()),
            is_auto_generated: false,
        },
        target_directory: target,
    }
}

#[tokio::test]
async fn selected_representation_controls_both_real_process_arguments_and_discovery()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let command = root.path().join("fixture-ytdlp.cmd");
    std::fs::write(
        &command,
        r#"@echo off
setlocal
:scan
if "%~1"=="" goto emit
if "%~1"=="--sub-format" set "fmt=%~2"
if "%~1"=="-P" set "target=%~2"
if "%~1"=="-o" for /f "tokens=1 delims=." %%A in ("%~2") do set "id=%%A"
shift
goto scan
:emit
echo %fmt%>"%target%\%id%.%fmt%"
"#
        .replace('\n', "\r\n"),
    )?;
    let adapter = YtDlpAdapter::new(vec![command]).with_timeout_ms(5000);
    for format in ["srt", "vtt"] {
        for cancellable in [false, true] {
            let target = root
                .path()
                .join(format!("{format} with spaces {cancellable}"));
            let input = request(target, format);
            let artifact = if cancellable {
                adapter
                    .download_subtitle_cancellable(input, &CancellationToken::new())
                    .await?
            } else {
                adapter.download_subtitle(input).await?
            };
            let ArtifactLocation::LocalPath(path) = artifact.location else {
                return Err("missing fixture output path".into());
            };
            assert_eq!(
                std::path::Path::new(&path)
                    .extension()
                    .and_then(|v| v.to_str()),
                Some(format)
            );
            assert_eq!(std::fs::read_to_string(path)?.trim(), format);
        }
    }
    Ok(())
}

#[tokio::test]
async fn unsupported_representation_is_rejected_before_directory_creation_or_spawn()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let adapter = YtDlpAdapter::new(vec![]);
    for cancellable in [false, true] {
        let target = root.path().join(format!("unsupported-{cancellable}"));
        let input = request(target.clone(), "json3");
        let outcome = if cancellable {
            adapter
                .download_subtitle_cancellable(input, &CancellationToken::new())
                .await
        } else {
            adapter.download_subtitle(input).await
        };
        assert!(matches!(outcome, Err(PortError::InvalidSource { .. })));
        assert!(!target.exists());
    }
    Ok(())
}
