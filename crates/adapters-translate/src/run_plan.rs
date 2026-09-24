use crate::source_snapshot;
use auralis_translation::{
    BlockPolicy, LanguagePair, ProgressSink, RetryPolicy, RunControl, RunId, SegmentId, SourceHash,
    TargetSegment, TranslateRunError, TranslationId, VerifiedRenderer,
};
use auralis_translation_formats::srt::{SrtRunError, SrtRunPlan};
use auralis_translation_formats::vtt::{VttRunError, VttRunPlan};
use auralis_translation_llamacpp::LlamaCppProvider;
use auralis_translation_sqlite::SegmentSpec;
use auralis_translation_sqlite::TranslateDb;
use ports::error::PortError;

pub(crate) enum DocumentPlan {
    Srt(SrtRunPlan),
    Vtt(VttRunPlan),
}

impl DocumentPlan {
    pub(crate) fn new(
        format: &str,
        source: &[u8],
        translation_id: TranslationId,
        run_id: RunId,
        pair: LanguagePair,
        policy: BlockPolicy,
    ) -> Result<Self, PortError> {
        match format {
            "srt" => SrtRunPlan::new(source, translation_id, run_id, pair, policy)
                .map(Self::Srt)
                .map_err(invalid_plan),
            "vtt" => VttRunPlan::new(source, translation_id, run_id, pair, policy)
                .map(Self::Vtt)
                .map_err(invalid_plan),
            _ => Err(PortError::Unsupported {
                message: "unsupported translation source format".into(),
            }),
        }
    }

    pub(crate) fn source_hash(&self) -> SourceHash {
        match self {
            Self::Srt(plan) => plan.source_hash(),
            Self::Vtt(plan) => plan.source_hash(),
        }
    }

    pub(crate) fn parser_version(&self) -> u32 {
        match self {
            Self::Srt(_) => SrtRunPlan::PARSER_VERSION,
            Self::Vtt(_) => VttRunPlan::PARSER_VERSION,
        }
    }

    pub(crate) fn policy_fingerprint(&self, policy: BlockPolicy) -> SourceHash {
        match self {
            Self::Srt(_) => SrtRunPlan::policy_fingerprint(policy),
            Self::Vtt(_) => VttRunPlan::policy_fingerprint(policy),
        }
    }

    pub(crate) fn blocks(&self) -> &[Vec<SegmentId>] {
        match self {
            Self::Srt(plan) => plan.blocks(),
            Self::Vtt(plan) => plan.blocks(),
        }
    }

    pub(crate) fn block_fingerprints(&self) -> Vec<SourceHash> {
        match self {
            Self::Srt(plan) => plan.block_fingerprints(),
            Self::Vtt(plan) => plan.block_fingerprints(),
        }
    }

    pub(crate) fn execute_with_policy(
        &self,
        provider: &LlamaCppProvider,
        database: &mut TranslateDb,
        progress: &mut impl ProgressSink,
        control: &impl RunControl,
        retry: RetryPolicy,
    ) -> Result<Vec<u8>, PortError> {
        match self {
            Self::Srt(plan) => plan
                .execute_with_policy(provider, database, progress, control, retry)
                .map_err(|error| match error {
                    SrtRunError::Translate(TranslateRunError::Paused) => PortError::Cancelled,
                    other => PortError::ExternalToolFailed {
                        tool: "Translate model execution".into(),
                        message: other.to_string(),
                    },
                }),
            Self::Vtt(plan) => plan
                .execute_with_policy(provider, database, progress, control, retry)
                .map_err(|error| match error {
                    VttRunError::Translate(TranslateRunError::Paused) => PortError::Cancelled,
                    other => PortError::ExternalToolFailed {
                        tool: "Translate model execution".into(),
                        message: other.to_string(),
                    },
                }),
        }
    }

    pub(crate) fn snapshot(&self) -> Result<Vec<SegmentSpec>, PortError> {
        source_snapshot::from_plan(self)
    }
}

impl VerifiedRenderer for DocumentPlan {
    type Error = PortError;

    fn source_hash(&self) -> SourceHash {
        DocumentPlan::source_hash(self)
    }

    fn render_selected(&self, selected: &[TargetSegment]) -> Result<Vec<u8>, Self::Error> {
        match self {
            Self::Srt(plan) => plan.render_selected(selected).map_err(invalid_plan),
            Self::Vtt(plan) => plan.render_selected(selected).map_err(invalid_plan),
        }
    }

    fn structural_evidence(&self) -> &'static str {
        match self {
            Self::Srt(_) => SrtRunPlan::STRUCTURAL_EVIDENCE,
            Self::Vtt(_) => VttRunPlan::STRUCTURAL_EVIDENCE,
        }
    }
}

fn invalid_plan(error: impl std::error::Error) -> PortError {
    PortError::InvalidSource {
        message: error.to_string(),
    }
}
