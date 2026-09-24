use crate::state::{RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository};
use adapters_translate::{
    FormatInspector, TranslateResultReader, TranslateRunControl, TranslateRunExecutor,
    TranslateRunRecovery, TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationUseCase, ExecuteTranslationUseCase, InspectTranslationSourceUseCase,
    PublishTranslationResultUseCase, ReconcileTranslationRunsUseCase,
    RecoverTranslationHostJobsUseCase, RunTranslationHostJobUseCase, StartTranslationJobUseCase,
    TranslationRunUseCase,
};
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_links::TranslationLinkStore;
use std::path::PathBuf;
use std::sync::Arc;

pub struct TranslationUseCases {
    pub inspect: Arc<InspectTranslationSourceUseCase>,
    pub begin: BeginTranslationUseCase,
    pub execute: Arc<ExecuteTranslationUseCase>,
    pub run_job: RunTranslationHostJobUseCase,
    pub run: TranslationRunUseCase,
    pub publish: PublishTranslationResultUseCase,
    pub reconcile: ReconcileTranslationRunsUseCase,
    pub start_job: StartTranslationJobUseCase,
    pub recover_jobs: RecoverTranslationHostJobsUseCase,
}

impl TranslationUseCases {
    pub fn new(
        projects: RuntimeProjectRepository,
        artifacts: RuntimeArtifactIndex,
        artifact_store: RuntimeArtifactStore,
        links: Arc<dyn TranslationLinkStore>,
        jobs: Arc<dyn TranslationHostJobStore>,
        database_path: PathBuf,
    ) -> Self {
        let inspect = Arc::new(InspectTranslationSourceUseCase::new(
            projects,
            artifacts,
            artifact_store.clone(),
            Arc::new(FormatInspector),
        ));
        let registrar = Arc::new(TranslateRunRegistrar::new(database_path.clone()));
        let control = Arc::new(TranslateRunControl::new(database_path.clone()));
        let execute = Arc::new(ExecuteTranslationUseCase::new(
            inspect.clone(),
            links.clone(),
            registrar.clone(),
            Arc::new(TranslateRunExecutor::new(database_path.clone())),
        ));
        Self {
            inspect: inspect.clone(),
            begin: BeginTranslationUseCase::new(inspect.clone(), links.clone(), registrar.clone()),
            execute: execute.clone(),
            run_job: RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), execute),
            run: TranslationRunUseCase::new(links.clone(), control.clone()),
            start_job: StartTranslationJobUseCase::new(links.clone(), control, jobs.clone()),
            recover_jobs: RecoverTranslationHostJobsUseCase::new(
                jobs,
                Arc::new(TranslateRunRecovery::new(database_path.clone())),
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
