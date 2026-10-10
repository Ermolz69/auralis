#[path = "support/translation_history_fixture.rs"]
mod fixture;

use domain::chrono::Utc;
use domain::project::ProjectId;
use fixture::HistoryFixture;
use ports::error::PortError;
use ports::translation_links::{StartTranslationRun, TranslationLinkStore};
use std::error::Error;
use uuid::Uuid;

#[tokio::test]
async fn concurrent_manual_choices_commit_one_revision_and_reject_the_loser()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoryFixture::new().await?;
    let first = fixture.ready(1).await?;
    let second = fixture.ready(2).await?;
    fixture.ready(3).await?;
    let before = fixture.link().await?;
    let (left, right) = tokio::join!(
        fixture
            .store
            .select_historical_result(fixture.selection(first.result_id, before.revision)),
        fixture
            .store
            .select_historical_result(fixture.selection(second.result_id, before.revision)),
    );
    let winner = match (left, right) {
        (Ok(winner), Err(PortError::Conflict { .. }))
        | (Err(PortError::Conflict { .. }), Ok(winner)) => winner,
        _ => return Err("concurrent history choices did not produce one committed winner".into()),
    };
    assert_eq!(winner.revision, before.revision + 1);
    assert_eq!(
        fixture.link().await?.selected_result_id,
        winner.selected_result_id
    );
    Ok(())
}

#[tokio::test]
async fn concurrent_staging_and_history_selection_keep_one_transaction_owner()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoryFixture::new().await?;
    let first = fixture.ready(1).await?;
    let second = fixture.ready(2).await?;
    let before = fixture.link().await?;
    let (selection, staging) = tokio::join!(
        fixture
            .store
            .select_historical_result(fixture.selection(first.result_id, before.revision)),
        fixture.stage(3, before.revision),
    );
    match (selection, staging) {
        (Ok(selected), Err(error)) => {
            assert!(matches!(
                error.downcast_ref::<PortError>(),
                Some(PortError::Conflict { .. })
            ));
            assert_eq!(selected.selected_result_id, Some(first.result_id));
            assert_eq!(selected.revision, before.revision + 1);
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM translation_publications WHERE result_revision = 3",
            )
            .fetch_one(&fixture.pool)
            .await?;
            assert_eq!(count, 0);
        }
        (Err(PortError::Busy { .. }), Ok(pending)) => {
            assert_eq!(
                fixture.link().await?.selected_result_id,
                Some(second.result_id)
            );
            assert_eq!(fixture.link().await?.revision, before.revision);
            assert_eq!(pending.expected_link_revision, before.revision);
        }
        _ => return Err("publication and historical choice lost transaction ownership".into()),
    }
    Ok(())
}

#[tokio::test]
async fn cursor_history_is_bounded_and_stable_across_new_publications() -> Result<(), Box<dyn Error>>
{
    let fixture = HistoryFixture::new().await?;
    let first = fixture.ready(1).await?;
    let second = fixture.ready(2).await?;
    let third = fixture.ready(3).await?;
    let page = fixture
        .store
        .list_ready_history(
            &fixture.intent.project_id,
            fixture.intent.translation_id,
            None,
            2,
        )
        .await?;
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.result_id)
            .collect::<Vec<_>>(),
        [third.result_id, second.result_id]
    );
    assert_eq!(page.next_cursor, Some(second.result_id));
    assert_eq!(page.ready_count, 3);
    assert_eq!(page.link.selected_result_id, Some(third.result_id));
    fixture.ready(4).await?;
    let older = fixture
        .store
        .list_ready_history(
            &fixture.intent.project_id,
            fixture.intent.translation_id,
            page.next_cursor,
            2,
        )
        .await?;
    assert_eq!(
        older
            .entries
            .iter()
            .map(|entry| entry.result_id)
            .collect::<Vec<_>>(),
        [first.result_id]
    );
    assert_eq!(older.next_cursor, None);
    assert_eq!(older.ready_count, 4);
    for limit in [0, 101] {
        assert!(
            fixture
                .store
                .list_ready_history(
                    &fixture.intent.project_id,
                    fixture.intent.translation_id,
                    None,
                    limit
                )
                .await
                .is_err()
        );
    }
    assert!(matches!(
        fixture
            .store
            .list_ready_history(&ProjectId::new(), fixture.intent.translation_id, None, 1)
            .await,
        Err(PortError::NotFound { .. })
    ));
    let foreign = HistoryFixture::new().await?.ready(1).await?;
    assert!(matches!(
        fixture
            .store
            .list_ready_history(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                Some(foreign.result_id),
                1
            )
            .await,
        Err(PortError::Conflict { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn explicit_selection_preserves_active_run_and_rejects_stale_or_foreign_choices()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoryFixture::new().await?;
    let first = fixture.ready(1).await?;
    let second = fixture.ready(2).await?;
    let before = fixture.link().await?;
    let active_run_id = Uuid::new_v4();
    let active = fixture
        .store
        .start_run(StartTranslationRun {
            translation_id: fixture.intent.translation_id,
            run_id: active_run_id,
            expected_active_run_id: None,
            expected_link_revision: before.revision,
            updated_at: Utc::now(),
        })
        .await?;
    let selected = fixture
        .store
        .select_historical_result(fixture.selection(first.result_id, active.revision))
        .await?;
    assert_eq!(selected.active_run_id, Some(active_run_id));
    assert_eq!(selected.selected_result_id, Some(first.result_id));
    assert_eq!(selected.output_artifact_id, Some(first.artifact_id));
    assert_eq!(selected.revision, active.revision + 1);
    let repeated = fixture
        .store
        .select_historical_result(fixture.selection(first.result_id, active.revision))
        .await?;
    assert_eq!(repeated.revision, selected.revision);
    assert!(matches!(
        fixture
            .store
            .select_historical_result(fixture.selection(second.result_id, active.revision))
            .await,
        Err(PortError::Conflict { .. })
    ));
    let mut foreign = fixture.selection(second.result_id, selected.revision);
    foreign.project_id = ProjectId::new();
    assert!(matches!(
        fixture.store.select_historical_result(foreign).await,
        Err(PortError::NotFound { .. })
    ));
    assert!(
        fixture
            .store
            .finalize_ready_publication(fixture.intent.translation_id, second.result_id, Utc::now())
            .await
            .is_err()
    );
    assert_eq!(
        fixture.link().await?.selected_result_id,
        Some(first.result_id)
    );
    Ok(())
}

#[tokio::test]
async fn pending_publication_and_manual_choice_have_deterministic_transaction_order()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoryFixture::new().await?;
    let first = fixture.ready(1).await?;
    let second = fixture.ready(2).await?;
    let current = fixture.link().await?;
    let pending = fixture.stage(3, current.revision).await?;
    assert!(matches!(
        fixture
            .store
            .select_historical_result(fixture.selection(first.result_id, current.revision))
            .await,
        Err(PortError::Busy { .. })
    ));
    assert_eq!(
        fixture.link().await?.selected_result_id,
        Some(second.result_id)
    );
    sqlx::query("UPDATE translation_publications SET state = 'failed' WHERE result_id = ?")
        .bind(pending.result_id.to_string())
        .execute(&fixture.pool)
        .await?;
    let selected = fixture
        .store
        .select_historical_result(fixture.selection(first.result_id, current.revision))
        .await?;
    assert!(fixture.stage(4, current.revision).await.is_err());
    assert_eq!(fixture.link().await?.revision, selected.revision);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM translation_publications WHERE result_revision = 4",
    )
    .fetch_one(&fixture.pool)
    .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn history_and_selection_reject_ineligible_publications_and_artifacts()
-> Result<(), Box<dyn Error>> {
    let fixture = HistoryFixture::new().await?;
    let first = fixture.ready(1).await?;
    let second = fixture.ready(2).await?;
    let current = fixture.link().await?;
    for (sql, value) in [
        (
            "UPDATE artifacts SET state = ? WHERE id = ?",
            "pending_finalize",
        ),
        (
            "UPDATE artifacts SET kind = ? WHERE id = ?",
            "OriginalSubtitle",
        ),
    ] {
        sqlx::query(sql)
            .bind(value)
            .bind(first.artifact_id.to_string())
            .execute(&fixture.pool)
            .await?;
        let page = fixture
            .store
            .list_ready_history(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                None,
                100,
            )
            .await?;
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].result_id, second.result_id);
        assert!(
            fixture
                .store
                .select_historical_result(fixture.selection(first.result_id, current.revision))
                .await
                .is_err()
        );
        sqlx::query(
            "UPDATE artifacts SET state = 'ready', kind = 'TranslatedTranscript' WHERE id = ?",
        )
        .bind(first.artifact_id.to_string())
        .execute(&fixture.pool)
        .await?;
    }
    for state in ["pending", "failed"] {
        sqlx::query("UPDATE translation_publications SET state = ? WHERE result_id = ?")
            .bind(state)
            .bind(first.result_id.to_string())
            .execute(&fixture.pool)
            .await?;
        assert_eq!(
            fixture
                .store
                .list_ready_history(
                    &fixture.intent.project_id,
                    fixture.intent.translation_id,
                    None,
                    100
                )
                .await?
                .entries
                .len(),
            1
        );
        assert!(
            fixture
                .store
                .select_historical_result(fixture.selection(first.result_id, current.revision))
                .await
                .is_err()
        );
    }
    sqlx::query("UPDATE translation_publications SET state = 'ready', source_sha256 = ? WHERE result_id = ?")
        .bind("b".repeat(64)).bind(first.result_id.to_string()).execute(&fixture.pool).await?;
    assert_eq!(
        fixture
            .store
            .list_ready_history(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                None,
                100
            )
            .await?
            .entries
            .len(),
        1
    );
    assert!(
        fixture
            .store
            .select_historical_result(fixture.selection(first.result_id, current.revision))
            .await
            .is_err()
    );
    assert_eq!(fixture.link().await?.revision, current.revision);
    sqlx::query("UPDATE translation_publications SET source_sha256 = ? WHERE result_id = ?")
        .bind(&fixture.intent.source_sha256)
        .bind(first.result_id.to_string())
        .execute(&fixture.pool)
        .await?;
    let foreign_project = ProjectId::new();
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'Foreign', 'Draft', 'now', 'now')")
        .bind(foreign_project.to_string()).execute(&fixture.pool).await?;
    sqlx::query("UPDATE artifacts SET project_id = ? WHERE id = ?")
        .bind(foreign_project.to_string())
        .bind(first.artifact_id.to_string())
        .execute(&fixture.pool)
        .await?;
    assert_eq!(
        fixture
            .store
            .list_ready_history(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                None,
                100
            )
            .await?
            .entries
            .len(),
        1
    );
    assert!(
        fixture
            .store
            .select_historical_result(fixture.selection(first.result_id, current.revision))
            .await
            .is_err()
    );
    Ok(())
}
