use super::{historical_control::UninterruptedRun, historical_provider::HistoricalProvider};
use auralis_translation::{
    BlockPolicy, LanguageCode, LanguagePair, ResultId, ReviewState, RunId, TranslationId,
};
use auralis_translation_formats::{srt::SrtRunPlan, vtt::VttRunPlan};
use auralis_translation_sqlite::{ResultSpec, RunSpec, SqliteConfig, TranslateDb};
use ports::translation_links::TranslationRunIntent;
use std::{error::Error, path::Path};
use uuid::Uuid;

pub fn seed(
    path: &Path,
    source: &[u8],
    format: &str,
    intent: &TranslationRunIntent,
) -> Result<Uuid, Box<dyn Error>> {
    let translation_id = TranslationId::new(intent.translation_id).ok_or("nil translation")?;
    let run_id = RunId::new(intent.run_id).ok_or("nil run")?;
    let policy = BlockPolicy::with_context(
        intent.max_target_segments as usize,
        intent.context_before_segments as usize,
        intent.context_after_segments as usize,
    )
    .ok_or("invalid policy")?;
    let pair = LanguagePair::new(LanguageCode::Chinese, LanguageCode::Russian)?;
    let mut database = TranslateDb::open(path, SqliteConfig::default())?;
    let result_id = ResultId::new(Uuid::new_v4()).ok_or("nil result")?;
    let mut run = RunSpec {
        translation_id,
        run_id,
        source_hash: auralis_translation::SourceHash::digest(source),
        profile_fingerprint: intent.profile_sha256.clone(),
        parser_version: intent.parser_version,
        policy_fingerprint: intent.policy_fingerprint.clone(),
        glossary_revision: None,
        blocks: Vec::new(),
    };
    let fingerprints = if format == "srt" {
        let plan = SrtRunPlan::new(source, translation_id, run_id, pair, policy)?;
        run.blocks = plan.blocks().to_vec();
        database.begin_attempt(&run, None)?;
        plan.execute_with_policy(
            &HistoricalProvider,
            &mut database,
            &mut UninterruptedRun,
            &UninterruptedRun,
            auralis_translation::RetryPolicy::default(),
        )?;
        database.commit_result(
            &ResultSpec {
                result_id,
                run_id,
                revision: 1,
                source_hash: run.source_hash,
                block_fingerprints: plan.block_fingerprints(),
                review_state: ReviewState::NeedsReview,
            },
            &plan,
        )?;
        plan.block_fingerprints()
    } else {
        let plan = VttRunPlan::new(source, translation_id, run_id, pair, policy)?;
        run.blocks = plan.blocks().to_vec();
        database.begin_attempt(&run, None)?;
        plan.execute_with_policy(
            &HistoricalProvider,
            &mut database,
            &mut UninterruptedRun,
            &UninterruptedRun,
            auralis_translation::RetryPolicy::default(),
        )?;
        database.commit_result(
            &ResultSpec {
                result_id,
                run_id,
                revision: 1,
                source_hash: run.source_hash,
                block_fingerprints: plan.block_fingerprints(),
                review_state: ReviewState::NeedsReview,
            },
            &plan,
        )?;
        plan.block_fingerprints()
    };
    assert!(!fingerprints.is_empty());
    Ok(result_id.get())
}
