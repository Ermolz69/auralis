use adapters_storage::{
    local::LocalArtifactStore,
    sqlite::{SqliteArtifactIndex, SqliteTranslationLinkStore, connect_sqlite},
};
use adapters_translate::{TranslateResultReader, TranslateRunControl};
use application::usecases::translation::{
    InspectTranslationSourceUseCase, PublishTranslationResultUseCase,
    RecoverTranslationPublicationsUseCase, SelectTranslationResultRequest,
    SelectTranslationResultUseCase,
};
use domain::project::ProjectId;
use ports::translation_links::TranslationLinkStore;
use std::{error::Error, path::PathBuf, sync::Arc};
use uuid::Uuid;

pub struct TranslationHistoryChecks {
    pub inspector: Arc<InspectTranslationSourceUseCase>,
    pub links: Arc<SqliteTranslationLinkStore>,
    pub artifacts: Arc<SqliteArtifactIndex>,
    pub files: Arc<LocalArtifactStore>,
    pub host_path: PathBuf,
    pub translate_path: PathBuf,
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub older_result_id: Uuid,
    pub latest_result_id: Uuid,
}

impl TranslationHistoryChecks {
    pub async fn verify(&self) -> Result<(), Box<dyn Error>> {
        let selector = SelectTranslationResultUseCase::new(
            self.inspector.clone(),
            self.links.clone(),
            Arc::new(TranslateResultReader::new(self.translate_path.clone())),
            self.artifacts.clone(),
            self.files.clone(),
        );
        let request = |result_id, expected_link_revision| SelectTranslationResultRequest {
            project_id: self.project_id.clone(),
            translation_id: self.translation_id,
            result_id,
            expected_link_revision,
        };
        let before = self
            .links
            .get(self.translation_id)
            .await?
            .ok_or("missing link")?;
        let history = self
            .links
            .list_ready_history(&self.project_id, self.translation_id, None, 100)
            .await?;
        assert_eq!(
            history
                .entries
                .iter()
                .map(|entry| entry.result_id)
                .collect::<Vec<_>>(),
            [self.latest_result_id, self.older_result_id]
        );
        let source = std::fs::read(&self.source_path)?;
        let output = std::fs::read(&self.output_path)?;
        let changed_source = String::from_utf8(source.clone())?.replace("你好", "您好");
        assert_ne!(source, changed_source.as_bytes());
        std::fs::write(&self.source_path, changed_source)?;
        let corrupted_source = selector
            .execute(request(self.older_result_id, before.revision))
            .await;
        std::fs::write(&self.source_path, &source)?;
        assert!(corrupted_source.is_err());

        let absent_source = self.source_path.with_extension("missing-for-test");
        std::fs::rename(&self.source_path, &absent_source)?;
        let missing_source = selector
            .execute(request(self.older_result_id, before.revision))
            .await;
        std::fs::rename(&absent_source, &self.source_path)?;
        assert!(missing_source.is_err());

        std::fs::write(&self.output_path, vec![b'x'; output.len()])?;
        let corrupted_output = selector
            .execute(request(self.older_result_id, before.revision))
            .await;
        std::fs::write(&self.output_path, &output)?;
        assert!(corrupted_output.is_err());
        let missing_path = self.output_path.with_extension("missing-for-test");
        std::fs::rename(&self.output_path, &missing_path)?;
        let missing_output = selector
            .execute(request(self.older_result_id, before.revision))
            .await;
        std::fs::rename(&missing_path, &self.output_path)?;
        assert!(missing_output.is_err());
        assert_eq!(
            self.links
                .get(self.translation_id)
                .await?
                .ok_or("missing link")?
                .revision,
            before.revision
        );

        let selected = selector
            .execute(request(self.older_result_id, before.revision))
            .await?;
        assert_eq!(selected.selected_result_id, Some(self.older_result_id));
        assert_eq!(selected.active_run_id, before.active_run_id);
        let repeated = selector
            .execute(request(self.older_result_id, before.revision))
            .await?;
        assert_eq!(selected.revision, repeated.revision);

        let reopened_pool = connect_sqlite(&self.host_path).await?;
        let reopened_links = Arc::new(SqliteTranslationLinkStore::new(reopened_pool.clone()));
        let publisher = PublishTranslationResultUseCase::new(
            self.inspector.clone(),
            reopened_links.clone(),
            Arc::new(TranslateResultReader::new(self.translate_path.clone())),
            self.files.clone(),
        );
        let recovery = RecoverTranslationPublicationsUseCase::new(
            reopened_links.clone(),
            Arc::new(TranslateRunControl::new(self.translate_path.clone())),
            publisher,
        );
        let outcomes = recovery.execute().await?;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].translation_id, self.translation_id);
        assert!(outcomes[0].outcome.is_ok());
        assert_eq!(
            reopened_links
                .get(self.translation_id)
                .await?
                .ok_or("missing reopened link")?
                .selected_result_id,
            Some(self.older_result_id)
        );
        assert_eq!(std::fs::read(&self.source_path)?, source);
        assert_eq!(std::fs::read(&self.output_path)?, output);
        selector
            .execute(request(self.latest_result_id, selected.revision))
            .await?;
        reopened_pool.close().await;
        Ok(())
    }
}
