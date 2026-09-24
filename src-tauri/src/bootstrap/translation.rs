use crate::state::{RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository};
use adapters_translate::{
    FormatInspector, TranslateResultReader, TranslateRunControl, TranslateRunExecutor,
    TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationUseCase, ExecuteTranslationUseCase, InspectTranslationSourceUseCase,
    PublishTranslationResultUseCase, ReconcileTranslationRunsUseCase, TranslationRunUseCase,
};
use ports::translation_links::TranslationLinkStore;
use std::path::PathBuf;
use std::sync::Arc;

pub struct TranslationUseCases {
    pub inspect: Arc<InspectTranslationSourceUseCase>,
    pub begin: BeginTranslationUseCase,
    pub execute: ExecuteTranslationUseCase,
    pub run: TranslationRunUseCase,
    pub publish: PublishTranslationResultUseCase,
    pub reconcile: ReconcileTranslationRunsUseCase,
}

impl TranslationUseCases {
    pub fn new(
        projects: RuntimeProjectRepository,
        artifacts: RuntimeArtifactIndex,
        artifact_store: RuntimeArtifactStore,
        links: Arc<dyn TranslationLinkStore>,
        database_path: PathBuf,
    ) -> Self {
        let inspect = Arc::new(InspectTranslationSourceUseCase::new(
            projects,
            artifacts,
            artifact_store.clone(),
            Arc::new(FormatInspector),
        ));
        let registrar = Arc::new(TranslateRunRegistrar::new(database_path.clone()));
        Self {
            inspect: inspect.clone(),
            begin: BeginTranslationUseCase::new(inspect.clone(), links.clone(), registrar.clone()),
            execute: ExecuteTranslationUseCase::new(
                inspect.clone(),
                links.clone(),
                registrar.clone(),
                Arc::new(TranslateRunExecutor::new(database_path.clone())),
            ),
            run: TranslationRunUseCase::new(
                links.clone(),
                Arc::new(TranslateRunControl::new(database_path.clone())),
            ),
            publish: PublishTranslationResultUseCase::new(
                inspect.clone(),
                links.clone(),
                Arc::new(TranslateResultReader::new(database_path)),
                artifact_store,
            ),
            reconcile: ReconcileTranslationRunsUseCase::new(inspect, links, registrar),
        }
    }
}
