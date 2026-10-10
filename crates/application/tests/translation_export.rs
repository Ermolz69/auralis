#[allow(dead_code)]
#[path = "support/historical_control.rs"]
mod historical_control;
#[allow(dead_code)]
#[path = "support/historical_fixture.rs"]
mod historical_fixture;
#[allow(dead_code)]
#[path = "support/historical_provider.rs"]
mod historical_provider;
#[allow(dead_code)]
#[path = "support/historical_seed.rs"]
mod historical_seed;

use application::usecases::translation::{
    ExportTranslationResultRequest, ExportTranslationResultUseCase,
};
use domain::project::ProjectId;
use historical_fixture::HistoricalFixture;
use ports::translation_inspection::TranslationSourceFormat;
use std::error::Error;

#[tokio::test]
async fn srt_history_exports_exact_immutable_versions() -> Result<(), Box<dyn Error>> {
    export_flow(TranslationSourceFormat::Srt).await
}

#[tokio::test]
async fn vtt_history_exports_exact_immutable_versions() -> Result<(), Box<dyn Error>> {
    export_flow(TranslationSourceFormat::Vtt).await
}

async fn export_flow(format: TranslationSourceFormat) -> Result<(), Box<dyn Error>> {
    let fixture = HistoricalFixture::new(format).await?;
    let export_dir = tempfile::tempdir()?;
    let exporter = ExportTranslationResultUseCase::new(
        fixture.verifier(),
        fixture.directory.path().to_path_buf(),
    );
    let extension = if format == TranslationSourceFormat::Srt {
        "srt"
    } else {
        "vtt"
    };
    let original_bytes = std::fs::read(&fixture.source_path)?;
    for result_id in [fixture.base_id, fixture.head_id] {
        let verified = fixture
            .verifier()
            .execute(
                fixture.project_id.clone(),
                fixture.translation_id,
                result_id,
            )
            .await?;
        let destination = export_dir.path().join(format!("{result_id}.{extension}"));
        let receipt = exporter
            .execute(ExportTranslationResultRequest {
                project_id: fixture.project_id.clone(),
                translation_id: fixture.translation_id,
                result_id,
                expected_result_sha256: verified.result.output_sha256.clone(),
                destination: destination.clone(),
            })
            .await?;
        assert_eq!(receipt.result_id, result_id);
        assert_eq!(receipt.result_sha256, verified.result.output_sha256);
        assert_eq!(
            receipt.size_bytes as usize,
            verified.result.output_bytes.len()
        );
        assert_eq!(receipt.destination, destination);
        let exported = std::fs::read(destination)?;
        assert_eq!(exported, verified.result.output_bytes);
        let cue_count = match format {
            TranslationSourceFormat::Srt => {
                auralis_translation_formats::srt::SrtDocument::parse(&exported)?
                    .segments()
                    .len()
            }
            TranslationSourceFormat::Vtt => {
                auralis_translation_formats::vtt::VttDocument::parse(&exported)?
                    .segments()
                    .len()
            }
        };
        assert_eq!(cue_count, 2);
    }
    assert_eq!(std::fs::read(&fixture.source_path)?, original_bytes);
    Ok(())
}

#[tokio::test]
async fn export_rejects_stale_digest_existing_file_owned_root_and_wrong_identity()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Srt).await?;
    let export_dir = tempfile::tempdir()?;
    let exporter = ExportTranslationResultUseCase::new(
        fixture.verifier(),
        fixture.directory.path().to_path_buf(),
    );
    let verified = fixture
        .verifier()
        .execute(
            fixture.project_id.clone(),
            fixture.translation_id,
            fixture.head_id,
        )
        .await?;
    let fresh = export_dir.path().join("fresh.srt");
    assert!(
        exporter
            .execute(ExportTranslationResultRequest {
                project_id: fixture.project_id.clone(),
                translation_id: fixture.translation_id,
                result_id: fixture.head_id,
                expected_result_sha256: "f".repeat(64),
                destination: fresh.clone(),
            })
            .await
            .is_err()
    );
    assert!(!fresh.exists());
    std::fs::write(&fresh, b"existing user file")?;
    let request = |project_id, destination| ExportTranslationResultRequest {
        project_id,
        translation_id: fixture.translation_id,
        result_id: fixture.head_id,
        expected_result_sha256: verified.result.output_sha256.clone(),
        destination,
    };
    assert!(
        exporter
            .execute(request(fixture.project_id.clone(), fresh.clone()))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&fresh)?, b"existing user file");
    assert_eq!(std::fs::read_dir(export_dir.path())?.count(), 1);
    assert!(
        exporter
            .execute(request(
                fixture.project_id.clone(),
                fixture.directory.path().join("owned.srt")
            ))
            .await
            .is_err()
    );
    assert!(!fixture.directory.path().join("owned.srt").exists());
    assert!(
        exporter
            .execute(request(
                fixture.project_id.clone(),
                export_dir.path().join("wrong.vtt")
            ))
            .await
            .is_err()
    );
    assert!(
        exporter
            .execute(request(
                ProjectId::new(),
                export_dir.path().join("foreign.srt")
            ))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&fixture.source_path)?, fixture.source);
    Ok(())
}
