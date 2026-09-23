pub(super) fn classify_cleanup_outcome(
    join_result: Result<ports::job_runtime_control::RuntimeTaskOutcome, tokio::task::JoinError>,
    was_aborted: bool,
) -> ports::job_runtime_control::RuntimeCleanupOutcome {
    use ports::job_runtime_control::RuntimeCleanupOutcome;

    match classify_join_result(join_result, was_aborted) {
        ClassifiedJoinResult::Completed => RuntimeCleanupOutcome::Completed,
        ClassifiedJoinResult::CooperativeCancelled => RuntimeCleanupOutcome::CooperativeCancelled,
        ClassifiedJoinResult::ForcedAborted => RuntimeCleanupOutcome::ForcedAborted,
        ClassifiedJoinResult::Panicked => RuntimeCleanupOutcome::Panicked,
        ClassifiedJoinResult::JoinFailed => RuntimeCleanupOutcome::JoinFailed,
    }
}

enum ClassifiedJoinResult {
    Completed,
    CooperativeCancelled,
    ForcedAborted,
    Panicked,
    JoinFailed,
}

fn classify_join_result(
    join_result: Result<ports::job_runtime_control::RuntimeTaskOutcome, tokio::task::JoinError>,
    was_aborted: bool,
) -> ClassifiedJoinResult {
    use ports::job_runtime_control::RuntimeTaskOutcome;

    match join_result {
        Ok(RuntimeTaskOutcome::Completed | RuntimeTaskOutcome::DeletedNoOp) => {
            ClassifiedJoinResult::Completed
        }
        Ok(RuntimeTaskOutcome::Cancelled) if was_aborted => ClassifiedJoinResult::ForcedAborted,
        Ok(RuntimeTaskOutcome::Cancelled) => ClassifiedJoinResult::CooperativeCancelled,
        Ok(RuntimeTaskOutcome::ApplicationFailed | RuntimeTaskOutcome::RecoveryRequired) => {
            ClassifiedJoinResult::JoinFailed
        }
        Ok(RuntimeTaskOutcome::Panicked) => ClassifiedJoinResult::Panicked,
        Err(error) if error.is_panic() => ClassifiedJoinResult::Panicked,
        Err(error) if error.is_cancelled() => ClassifiedJoinResult::ForcedAborted,
        Err(_) => ClassifiedJoinResult::JoinFailed,
    }
}

pub(super) fn classify_outcome(
    join_res: Result<ports::job_runtime_control::RuntimeTaskOutcome, tokio::task::JoinError>,
    report: &mut ports::job_runtime_control::RuntimeShutdownReport,
    was_aborted: bool,
) {
    match classify_join_result(join_res, was_aborted) {
        ClassifiedJoinResult::Completed => {
            report.completed_count += 1;
        }
        ClassifiedJoinResult::CooperativeCancelled => {
            report.cooperative_cancelled_count += 1;
        }
        ClassifiedJoinResult::ForcedAborted => {
            report.forced_aborted_count += 1;
        }
        ClassifiedJoinResult::Panicked => {
            report.panicked_count += 1;
        }
        ClassifiedJoinResult::JoinFailed => {
            report.join_failed_count += 1;
        }
    }
}
