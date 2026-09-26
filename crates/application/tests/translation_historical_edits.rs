#[path = "support/historical_control.rs"]
mod historical_control;
#[path = "support/historical_fixture.rs"]
mod historical_fixture;
#[path = "support/historical_provider.rs"]
mod historical_provider;
#[path = "support/historical_seed.rs"]
mod historical_seed;
#[path = "translation_historical_edits/observations.rs"]
mod observations;

use application::usecases::translation::{
    EditHistoricalTranslationRequest, SelectTranslationResultRequest,
    SelectTranslationResultUseCase,
};
use historical_fixture::HistoricalFixture;
use ports::{
    translation_edit_intents::TranslationEditIntentStore,
    translation_inspection::TranslationSourceFormat,
    translation_links::{TranslationLinkStore, TranslationPublicationState},
};
use std::{error::Error, sync::Arc};
use uuid::Uuid;

fn copy_request(request: &EditHistoricalTranslationRequest) -> EditHistoricalTranslationRequest {
    EditHistoricalTranslationRequest {
        project_id: request.project_id.clone(),
        translation_id: request.translation_id,
        base_result_id: request.base_result_id,
        result_id: request.result_id,
        expected_head_result_id: request.expected_head_result_id,
        expected_link_revision: request.expected_link_revision,
        segment_id: request.segment_id,
        lines: request.lines.clone(),
    }
}

#[tokio::test]
async fn corrupt_base_and_missing_edit_provenance_cannot_admit_or_publish_a_branch()
-> Result<(), Box<dyn Error>> {
    use sqlx::Connection;
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Vtt).await?;
    let request = fixture.request().await?;
    let base = fixture
        .verifier()
        .execute(
            fixture.project_id.clone(),
            fixture.translation_id,
            fixture.base_id,
        )
        .await?;
    let publication = fixture
        .links
        .get_publication(fixture.translation_id, fixture.base_id)
        .await?
        .ok_or("missing base publication")?;
    use ports::{artifact_index::ArtifactIndex, storage::ArtifactStore};
    let artifact = fixture
        .artifacts
        .list_by_project_and_kind(
            &fixture.project_id,
            domain::media::ArtifactKind::TranslatedTranscript,
        )
        .await?
        .into_iter()
        .find(|artifact| artifact.id == publication.artifact_id)
        .ok_or("missing base artifact")?;
    let path = fixture.files.resolve_artifact(&artifact).await?;
    std::fs::write(&path, b"corrupt")?;
    assert!(
        fixture
            .editor()
            .execute(copy_request(&request))
            .await
            .is_err()
    );
    assert!(fixture.intents.get(request.result_id).await?.is_none());
    std::fs::write(&path, &base.result.output_bytes)?;
    fixture.editor().execute(copy_request(&request)).await?;
    let options = sqlx::sqlite::SqliteConnectOptions::new().filename(&fixture.translate_path);
    let mut connection = sqlx::SqliteConnection::connect_with(&options).await?;
    sqlx::query("DELETE FROM result_edit_provenance WHERE result_id = ?")
        .bind(request.result_id.to_string())
        .execute(&mut connection)
        .await?;
    assert!(fixture.publish(request.result_id).await.is_err());
    assert!(
        fixture
            .links
            .get_publication(fixture.translation_id, request.result_id)
            .await?
            .is_none()
    );
    sqlx::query("INSERT INTO result_edit_provenance (result_id, base_result_id, observed_head_result_id, segment_id) VALUES (?, ?, ?, 1)")
        .bind(request.result_id.to_string()).bind(fixture.base_id.to_string()).bind(fixture.head_id.to_string()).execute(&mut connection).await?;
    connection.close().await?;
    fixture.publish(request.result_id).await?;
    fixture.flush().await?;
    assert_eq!(std::fs::read(&path)?, base.result.output_bytes);
    assert_eq!(std::fs::read(&fixture.source_path)?, fixture.source);
    Ok(())
}

#[tokio::test]
async fn branch_gap_recovery_preserves_older_text_and_another_unfinished_run_for_both_formats()
-> Result<(), Box<dyn Error>> {
    for format in [TranslationSourceFormat::Srt, TranslationSourceFormat::Vtt] {
        let mut fixture = HistoricalFixture::new(format).await?;
        let request = fixture.request().await?;
        let result_id = request.result_id;
        fixture.editor().execute(copy_request(&request)).await?;
        assert!(
            fixture
                .links
                .get_publication(fixture.translation_id, result_id)
                .await?
                .is_none()
        );
        let observed = fixture
            .links
            .get(fixture.translation_id)
            .await?
            .ok_or("missing link")?;
        SelectTranslationResultUseCase::new(
            fixture.inspector(),
            fixture.links.clone(),
            Arc::new(adapters_translate::TranslateResultReader::new(
                fixture.translate_path.clone(),
            )),
            fixture.artifacts.clone(),
            fixture.files.clone(),
        )
        .execute(SelectTranslationResultRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.base_id,
            expected_link_revision: observed.revision,
        })
        .await?;
        let active = fixture.start_other_run().await?;
        let before = fixture
            .links
            .get(fixture.translation_id)
            .await?
            .ok_or("missing link")?;
        fixture.reopen().await?;
        let outcomes = fixture.recovery().execute().await?;
        assert!(outcomes.iter().all(|item| item.outcome.is_ok()));
        fixture.flush().await?;
        assert_eq!(
            fixture
                .links
                .get(fixture.translation_id)
                .await?
                .ok_or("missing link")?,
            before
        );
        assert_eq!(before.active_run_id, Some(active));
        let ready = fixture
            .links
            .get_publication(fixture.translation_id, result_id)
            .await?
            .ok_or("missing branch")?;
        assert_eq!(ready.state, TranslationPublicationState::Ready);
        assert_eq!(ready.result_revision, 3);
        let verified = fixture
            .verifier()
            .execute(
                fixture.project_id.clone(),
                fixture.translation_id,
                result_id,
            )
            .await?;
        assert_eq!(verified.result.segments[0].translated_lines, request.lines);
        assert_eq!(
            verified.result.segments[1].translated_lines,
            ["Исходный перевод 2."]
        );
        let expected = String::from_utf8(fixture.source.clone())?
            .replace("你好。", &request.lines[0])
            .replace("再见。", "Исходный перевод 2.");
        assert_eq!(verified.result.output_bytes, expected.as_bytes());
        assert_eq!(std::fs::read(&fixture.source_path)?, fixture.source);
        let ancestry = verified.result.edit_provenance.ok_or("missing origin")?;
        assert_eq!(ancestry.base_result_id, fixture.base_id);
        assert_eq!(ancestry.observed_head_result_id, fixture.head_id);
        fixture.editor().execute(copy_request(&request)).await?;
        fixture.publish(result_id).await?;
        fixture.reopen().await?;
        assert!(
            fixture
                .recovery()
                .execute()
                .await?
                .iter()
                .all(|item| item.outcome.is_ok())
        );
        fixture.flush().await?;
        assert_eq!(
            fixture
                .links
                .get(fixture.translation_id)
                .await?
                .ok_or("missing link")?,
            before
        );
        assert_eq!(
            fixture
                .intents
                .get(result_id)
                .await?
                .ok_or("missing intent")?
                .run_id,
            fixture.run_id
        );
    }
    Ok(())
}

#[tokio::test]
async fn concurrent_branch_saves_have_one_head_winner_and_exact_retries_survive_later_revisions()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Srt).await?;
    let first = fixture.request().await?;
    let second = fixture.request().await?;
    let editor = fixture.editor();
    let (left, right) = tokio::join!(
        editor.execute(copy_request(&first)),
        editor.execute(copy_request(&second))
    );
    assert_ne!(left.is_ok(), right.is_ok());
    let winner = if left.is_ok() { first } else { second };
    assert_eq!(fixture.intents.list_unpublished(None, 100).await?.len(), 2);
    fixture.publish(winner.result_id).await?;
    fixture.flush().await?;
    let selected = fixture
        .links
        .get(fixture.translation_id)
        .await?
        .ok_or("missing link")?;
    assert_eq!(selected.selected_result_id, Some(winner.result_id));
    let mut later = fixture.request().await?;
    later.expected_head_result_id = winner.result_id;
    later.lines = vec!["Следующая историческая правка.".into()];
    fixture.editor().execute(copy_request(&later)).await?;
    fixture.editor().execute(copy_request(&winner)).await?;
    let mut changed = copy_request(&winner);
    changed.lines = vec!["Чужой запрос с прежним ID.".into()];
    assert!(fixture.editor().execute(changed).await.is_err());
    fixture.publish(later.result_id).await?;
    fixture.flush().await?;
    assert_eq!(
        fixture
            .verifier()
            .execute(
                fixture.project_id.clone(),
                fixture.translation_id,
                later.result_id
            )
            .await?
            .result
            .result_revision,
        4
    );
    assert_eq!(std::fs::read(&fixture.source_path)?, fixture.source);
    Ok(())
}

#[tokio::test]
async fn fingerprint_and_frozen_profile_mismatches_never_stage_a_manual_artifact()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Srt).await?;
    let request = fixture.request().await?;
    fixture.editor().execute(copy_request(&request)).await?;
    let saved = fixture
        .intents
        .get(request.result_id)
        .await?
        .ok_or("missing intent")?;
    sqlx::query("UPDATE translation_edit_intents SET request_sha256 = ? WHERE result_id = ?")
        .bind("0".repeat(64))
        .bind(request.result_id.to_string())
        .execute(&fixture.pool)
        .await?;
    assert!(fixture.publish(request.result_id).await.is_err());
    sqlx::query("UPDATE translation_edit_intents SET request_sha256 = ? WHERE result_id = ?")
        .bind(&saved.request_sha256)
        .bind(request.result_id.to_string())
        .execute(&fixture.pool)
        .await?;
    let frozen = fixture
        .links
        .run_intent(fixture.run_id)
        .await?
        .ok_or("missing run")?;
    sqlx::query("UPDATE translation_run_intents SET profile_sha256 = ? WHERE run_id = ?")
        .bind("0".repeat(64))
        .bind(fixture.run_id.to_string())
        .execute(&fixture.pool)
        .await?;
    assert!(fixture.publish(request.result_id).await.is_err());
    assert!(
        fixture
            .links
            .get_publication(fixture.translation_id, request.result_id)
            .await?
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM outbox_messages WHERE status != 'done'")
            .fetch_one(&fixture.pool)
            .await?,
        0
    );
    sqlx::query("UPDATE translation_run_intents SET profile_sha256 = ? WHERE run_id = ?")
        .bind(&frozen.profile_sha256)
        .bind(fixture.run_id.to_string())
        .execute(&fixture.pool)
        .await?;
    fixture.publish(request.result_id).await?;
    sqlx::query("UPDATE translation_publications SET result_revision = 1 WHERE result_id = ?")
        .bind(request.result_id.to_string())
        .execute(&fixture.pool)
        .await?;
    assert!(fixture.publish(request.result_id).await.is_err());
    sqlx::query("UPDATE translation_publications SET result_revision = 3 WHERE result_id = ?")
        .bind(request.result_id.to_string())
        .execute(&fixture.pool)
        .await?;
    fixture.flush().await?;
    assert_eq!(
        fixture
            .links
            .get_publication(fixture.translation_id, request.result_id)
            .await?
            .ok_or("missing branch")?
            .state,
        TranslationPublicationState::Ready
    );
    Ok(())
}

#[tokio::test]
async fn interrupted_intents_do_not_starve_a_later_committed_result_across_recovery_pages()
-> Result<(), Box<dyn Error>> {
    let mut fixture = HistoricalFixture::new(TranslationSourceFormat::Srt).await?;
    let interrupted = fixture.request().await?;
    // The stale head fails after journal admission, leaving an actual interrupted intent.
    let mut stale = copy_request(&interrupted);
    stale.expected_head_result_id = fixture.base_id;
    assert!(fixture.editor().execute(stale).await.is_err());
    let template = fixture
        .intents
        .get(interrupted.result_id)
        .await?
        .ok_or("missing interrupted intent")?;
    for _ in 0..100 {
        let mut intent = template.clone();
        intent.result_id = Uuid::new_v4();
        intent.created_at = domain::chrono::Utc::now();
        fixture.intents.record(intent).await?;
    }
    let request = fixture.request().await?;
    fixture.editor().execute(copy_request(&request)).await?;
    fixture.reopen().await?;
    let outcomes = fixture.recovery().execute().await?;
    assert_eq!(
        outcomes.iter().filter(|item| item.outcome.is_err()).count(),
        101
    );
    assert!(
        fixture
            .links
            .get_publication(fixture.translation_id, request.result_id)
            .await?
            .is_some()
    );
    fixture.flush().await?;
    assert_eq!(
        fixture
            .links
            .get(fixture.translation_id)
            .await?
            .ok_or("missing link")?
            .selected_result_id,
        Some(request.result_id)
    );
    assert!(
        fixture
            .links
            .get_publication(fixture.translation_id, interrupted.result_id)
            .await?
            .is_none()
    );
    Ok(())
}
