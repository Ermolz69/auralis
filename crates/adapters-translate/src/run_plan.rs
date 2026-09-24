use crate::source_snapshot;
use auralis_translation::{BlockPolicy, LanguagePair, RunId, SegmentId, SourceHash, TranslationId};
use auralis_translation_formats::srt::SrtRunPlan;
use auralis_translation_formats::vtt::VttRunPlan;
use auralis_translation_sqlite::SegmentSpec;
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

    pub(crate) fn snapshot(&self) -> Result<Vec<SegmentSpec>, PortError> {
        source_snapshot::from_plan(self)
    }
}

fn invalid_plan(error: impl std::error::Error) -> PortError {
    PortError::InvalidSource {
        message: error.to_string(),
    }
}
