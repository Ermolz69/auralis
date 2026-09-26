use crate::state::{
    RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository, RuntimeStorageUnitOfWork,
};
use adapters_translate::{
    FormatInspector, TranslateResultEditor, TranslateResultReader, TranslateRunControl,
    TranslateRunExecutor, TranslateRunRecovery, TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationUseCase, EditTranslationResultUseCase, ExecuteTranslationUseCase,
    GetTranslationComparisonUseCase, GetTranslationRunStatusUseCase,
    ImportTranslationSourceUseCase, InspectTranslationSourceUseCase,
    ListProjectTranslationsUseCase, ListTranslationHistoryUseCase, PublishTranslationResultUseCase,
    ReconcileTranslationRunsUseCase, RecoverTranslationHostJobsUseCase,
    RecoverTranslationPublicationsUseCase, RunManagedTranslationUseCase,
    RunTranslationHostJobUseCase, SelectTranslationResultUseCase, StartTranslationJobUseCase,
    TranslationRunUseCase,
};
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_links::TranslationLinkStore;
use ports::translation_model_runtime::TranslationModelRuntimePort;
use ports::translation_run_control::TranslationRunControlPort;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) const EXPERIMENTAL_CHECKED_CHINESE_PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
);

pub struct TranslationUseCases {
    control: Arc<dyn TranslationRunControlPort>,
    pub jobs: Arc<dyn TranslationHostJobStore>,
    pub inspect: Arc<InspectTranslationSourceUseCase>,
    pub import_source: ImportTranslationSourceUseCase,
    pub list: ListProjectTranslationsUseCase,
    pub comparison: GetTranslationComparisonUseCase,
    pub history: ListTranslationHistoryUseCase,
    pub select_result: SelectTranslationResultUseCase,
    pub begin: BeginTranslationUseCase,
    pub edit: EditTranslationResultUseCase,
    pub execute: Arc<ExecuteTranslationUseCase>,
    pub run_job: RunTranslationHostJobUseCase,
    pub run: TranslationRunUseCase,
    pub status: GetTranslationRunStatusUseCase,
    pub publish: PublishTranslationResultUseCase,
    pub reconcile: ReconcileTranslationRunsUseCase,
    pub start_job: StartTranslationJobUseCase,
    pub recover_jobs: RecoverTranslationHostJobsUseCase,
    pub recover_publications: RecoverTranslationPublicationsUseCase,
    links: Arc<dyn TranslationLinkStore>,
}

impl TranslationUseCases {
    pub fn new(
        projects: RuntimeProjectRepository,
        artifacts: RuntimeArtifactIndex,
        artifact_store: RuntimeArtifactStore,
        storage_uow: RuntimeStorageUnitOfWork,
        links: Arc<dyn TranslationLinkStore>,
        jobs: Arc<dyn TranslationHostJobStore>,
        database_path: PathBuf,
    ) -> Self {
        let inspect = Arc::new(InspectTranslationSourceUseCase::new(
            projects.clone(),
            artifacts.clone(),
            artifact_store.clone(),
            Arc::new(FormatInspector),
        ));
        let import_source = ImportTranslationSourceUseCase::new(
            projects.clone(),
            artifact_store.clone(),
            storage_uow,
            Arc::new(FormatInspector),
        );
        let registrar = Arc::new(TranslateRunRegistrar::new(database_path.clone()));
        let control = Arc::new(TranslateRunControl::new(database_path.clone()));
        let execute = Arc::new(ExecuteTranslationUseCase::new(
            inspect.clone(),
            links.clone(),
            registrar.clone(),
            Arc::new(TranslateRunExecutor::new(database_path.clone())),
        ));
        let run_job =
            RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), execute.clone());
        let start_job =
            StartTranslationJobUseCase::new(links.clone(), control.clone(), jobs.clone());
        let publish = PublishTranslationResultUseCase::new(
            inspect.clone(),
            links.clone(),
            Arc::new(TranslateResultReader::new(database_path.clone())),
            artifact_store.clone(),
        );
        Self {
            control: control.clone(),
            jobs: jobs.clone(),
            inspect: inspect.clone(),
            import_source,
            list: ListProjectTranslationsUseCase::new(projects, links.clone()),
            history: ListTranslationHistoryUseCase::new(links.clone()),
            select_result: SelectTranslationResultUseCase::new(
                inspect.clone(),
                links.clone(),
                Arc::new(TranslateResultReader::new(database_path.clone())),
                artifacts,
                artifact_store.clone(),
            ),
            comparison: GetTranslationComparisonUseCase::new(
                inspect.clone(),
                links.clone(),
                Arc::new(TranslateResultReader::new(database_path.clone())),
            ),
            begin: BeginTranslationUseCase::new(inspect.clone(), links.clone(), registrar.clone()),
            edit: EditTranslationResultUseCase::new(
                inspect.clone(),
                links.clone(),
                Arc::new(TranslateResultEditor::new(database_path.clone())),
            ),
            execute: execute.clone(),
            run_job,
            run: TranslationRunUseCase::new(links.clone(), control.clone()),
            status: GetTranslationRunStatusUseCase::new(
                TranslationRunUseCase::new(links.clone(), control.clone()),
                jobs.clone(),
            ),
            start_job,
            recover_jobs: RecoverTranslationHostJobsUseCase::new(
                jobs,
                Arc::new(TranslateRunRecovery::new(database_path.clone())),
            ),
            recover_publications: RecoverTranslationPublicationsUseCase::new(
                links.clone(),
                control.clone(),
                publish.clone(),
            ),
            publish,
            reconcile: ReconcileTranslationRunsUseCase::new(inspect, links.clone(), registrar),
            links,
        }
    }

    pub fn managed_for_runtime(
        &self,
        runtime: Arc<dyn TranslationModelRuntimePort>,
    ) -> RunManagedTranslationUseCase {
        RunManagedTranslationUseCase::new(
            self.links.clone(),
            runtime,
            self.control.clone(),
            self.start_job.clone(),
            self.run_job.clone(),
            self.publish.clone(),
        )
    }
}
