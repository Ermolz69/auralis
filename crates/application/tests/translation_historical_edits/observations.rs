use crate::historical_fixture::HistoricalFixture;
use adapters_translate::{TranslateResultReader, TranslateRunControl};
use application::usecases::translation::{
    GetHistoricalTranslationEditContextRequest, GetHistoricalTranslationEditContextUseCase,
    GetTranslationPublicationRequest, GetTranslationPublicationUseCase,
    SelectTranslationResultRequest, SelectTranslationResultUseCase,
};
use ports::{
    translation_inspection::TranslationSourceFormat,
    translation_links::{TranslationLinkStore, TranslationPublicationState},
};
use std::{error::Error, sync::Arc};
use uuid::Uuid;

fn context(fixture: &HistoricalFixture) -> GetHistoricalTranslationEditContextUseCase {
    GetHistoricalTranslationEditContextUseCase::new(
        fixture.verifier(),
        Arc::new(TranslateRunControl::new(fixture.translate_path.clone())),
        Arc::new(TranslateResultReader::new(fixture.translate_path.clone())),
    )
}

fn context_request(fixture: &HistoricalFixture) -> GetHistoricalTranslationEditContextRequest {
    GetHistoricalTranslationEditContextRequest {
        project_id: fixture.project_id.clone(),
        translation_id: fixture.translation_id,
        base_result_id: fixture.base_id,
    }
}

#[tokio::test]
async fn context_observes_the_base_run_despite_another_unfinished_run_and_rejects_a_stale_head()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Vtt).await?;
    let active = fixture.start_other_run().await?;
    let observed = context(&fixture).execute(context_request(&fixture)).await?;
    assert_eq!(observed.run_id, fixture.run_id);
    assert_ne!(observed.run_id, active);
    assert_eq!(observed.base_result_id, fixture.base_id);
    assert_eq!(observed.base_revision, 1);
    assert_eq!(observed.expected_head_result_id, fixture.head_id);
    assert_eq!(observed.head_revision, 2);
    let mut first = fixture.request().await?;
    first.expected_head_result_id = observed.expected_head_result_id;
    first.expected_link_revision = observed.expected_link_revision;
    fixture
        .editor()
        .execute(crate::copy_request(&first))
        .await?;
    let stale = fixture.request().await?;
    assert!(fixture.editor().execute(stale).await.is_err());
    let refreshed = context(&fixture).execute(context_request(&fixture)).await?;
    assert_eq!(refreshed.expected_head_result_id, first.result_id);
    assert_eq!(refreshed.head_revision, 3);
    assert_eq!(
        refreshed.expected_link_revision,
        observed.expected_link_revision
    );
    let mut next = fixture.request().await?;
    next.expected_head_result_id = refreshed.expected_head_result_id;
    fixture.editor().execute(crate::copy_request(&next)).await?;
    fixture.publish(next.result_id).await?;
    fixture.flush().await?;
    let view = fixture
        .links
        .publication_view(&fixture.project_id, fixture.translation_id, next.result_id)
        .await?;
    assert_eq!(view.publication.state, TranslationPublicationState::Ready);
    assert_eq!(view.link.active_run_id, Some(active));
    Ok(())
}

#[tokio::test]
async fn context_requires_a_ready_owned_base_and_frozen_profile() -> Result<(), Box<dyn Error>> {
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Srt).await?;
    let mut foreign = context_request(&fixture);
    foreign.project_id = domain::project::ProjectId::new();
    assert!(context(&fixture).execute(foreign).await.is_err());
    let request = fixture.request().await?;
    fixture
        .editor()
        .execute(crate::copy_request(&request))
        .await?;
    let mut unpublished = context_request(&fixture);
    unpublished.base_result_id = request.result_id;
    assert!(context(&fixture).execute(unpublished).await.is_err());
    sqlx::query("UPDATE translation_run_intents SET profile_sha256 = ? WHERE run_id = ?")
        .bind("0".repeat(64))
        .bind(fixture.run_id.to_string())
        .execute(&fixture.pool)
        .await?;
    assert!(
        context(&fixture)
            .execute(context_request(&fixture))
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn publication_observation_distinguishes_pending_attached_and_later_detached_versions()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Srt).await?;
    let read = GetTranslationPublicationUseCase::new(fixture.links.clone());
    let request = fixture.request().await?;
    fixture
        .editor()
        .execute(crate::copy_request(&request))
        .await?;
    fixture.publish(request.result_id).await?;
    let observed = read
        .execute(GetTranslationPublicationRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: request.result_id,
        })
        .await?;
    assert_eq!(
        observed.publication.state,
        TranslationPublicationState::Pending
    );
    assert_eq!(observed.link.selected_result_id, Some(fixture.head_id));
    // Finalization and repeated reads use independent pooled connections to the same real SQLite file.
    let (flushed, reads) = tokio::join!(fixture.flush(), async {
        for _ in 0..20 {
            let view = fixture
                .links
                .publication_view(
                    &fixture.project_id,
                    fixture.translation_id,
                    request.result_id,
                )
                .await?;
            match view.publication.state {
                TranslationPublicationState::Pending => {
                    assert_eq!(view.link.selected_result_id, Some(fixture.head_id))
                }
                TranslationPublicationState::Ready => {
                    assert_eq!(view.link.selected_result_id, Some(request.result_id));
                    assert_eq!(
                        view.link.output_artifact_id,
                        Some(view.publication.artifact_id)
                    );
                }
                TranslationPublicationState::Failed => panic!("unexpected failed publication"),
            }
        }
        Ok::<_, Box<dyn Error>>(())
    });
    flushed?;
    reads?;
    let ready = fixture
        .links
        .publication_view(
            &fixture.project_id,
            fixture.translation_id,
            request.result_id,
        )
        .await?;
    assert_eq!(ready.publication.state, TranslationPublicationState::Ready);
    assert_eq!(ready.link.selected_result_id, Some(request.result_id));
    SelectTranslationResultUseCase::new(
        fixture.inspector(),
        fixture.links.clone(),
        Arc::new(TranslateResultReader::new(fixture.translate_path.clone())),
        fixture.artifacts.clone(),
        fixture.files.clone(),
    )
    .execute(SelectTranslationResultRequest {
        project_id: fixture.project_id.clone(),
        translation_id: fixture.translation_id,
        result_id: fixture.base_id,
        expected_link_revision: ready.link.revision,
    })
    .await?;
    let detached = fixture
        .links
        .publication_view(
            &fixture.project_id,
            fixture.translation_id,
            request.result_id,
        )
        .await?;
    assert_eq!(
        detached.publication.state,
        TranslationPublicationState::Ready
    );
    assert_eq!(detached.link.selected_result_id, Some(fixture.base_id));
    assert!(
        fixture
            .links
            .publication_view(
                &domain::project::ProjectId::new(),
                fixture.translation_id,
                request.result_id
            )
            .await
            .is_err()
    );
    assert!(
        fixture
            .links
            .publication_view(&fixture.project_id, fixture.translation_id, Uuid::new_v4())
            .await
            .is_err()
    );
    sqlx::query("UPDATE artifacts SET state = 'pending_finalize' WHERE id = ?")
        .bind(detached.publication.artifact_id.to_string())
        .execute(&fixture.pool)
        .await?;
    assert!(
        fixture
            .links
            .publication_view(
                &fixture.project_id,
                fixture.translation_id,
                request.result_id
            )
            .await
            .is_err()
    );
    Ok(())
}
