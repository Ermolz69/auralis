use super::TranslationAdmissionPolicy;
use crate::error::ApplicationError;
use ports::{
    cancellation::CancellationToken,
    error::PortError,
    translation_links::{TranslationLink, TranslationLinkStore, TranslationRunIntent},
    translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort},
    translation_run_control::TranslationRunControlPort,
    translation_start_guard::TranslationStartGuard,
};

pub(super) async fn acquire(
    runtime: &dyn TranslationModelRuntimePort,
    links: &dyn TranslationLinkStore,
    control: &dyn TranslationRunControlPort,
    intent: &TranslationRunIntent,
    link: &TranslationLink,
    guard: TranslationStartGuard,
    policy: TranslationAdmissionPolicy,
) -> Result<Box<dyn TranslationModelLease>, ApplicationError> {
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let acquisition = runtime.acquire_with_control(intent, cancellation.clone());
    tokio::pin!(acquisition);
    let mut poll = tokio::time::interval(policy.poll_interval());
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let outcome = loop {
        tokio::select! {
            biased;
            _ = poll.tick() => {
                if let Err(error) = check(links, control, link, guard).await {
                    cancellation.cancel();
                    if let Err(cleanup @ PortError::CleanupFailed { .. }) = acquisition.await {
                        return Err(cleanup.into());
                    }
                    return Err(error);
                }
            }
            outcome = &mut acquisition => break outcome,
        }
    };
    let admission = check(links, control, link, guard).await;
    match outcome {
        Err(cleanup @ PortError::CleanupFailed { .. }) => Err(cleanup.into()),
        outcome => {
            admission?;
            outcome.map_err(Into::into)
        }
    }
}

async fn check(
    links: &dyn TranslationLinkStore,
    control: &dyn TranslationRunControlPort,
    link: &TranslationLink,
    guard: TranslationStartGuard,
) -> Result<(), ApplicationError> {
    let current = links
        .get(link.translation_id)
        .await?
        .ok_or(PortError::Cancelled)?;
    if current.project_id != link.project_id
        || current.active_run_id != Some(guard.run_id)
        || control.capture_start(&current).await? != guard
    {
        return Err(PortError::Cancelled.into());
    }
    Ok(())
}
