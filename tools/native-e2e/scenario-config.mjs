export function readScenarioConfig(env = process.env) {
  const translationEnabled = env.AURALIS_NATIVE_E2E_TRANSLATION === '1';
  const casterEnabled = env.AURALIS_NATIVE_E2E_CASTER === '1';
  if (casterEnabled && translationEnabled)
    throw new Error('Caster native E2E runs without Translate');
  const translationEventsEnabled = env.AURALIS_NATIVE_E2E_TRANSLATION_EVENTS === '1';
  const translationHistoryEnabled = env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY === '1';
  const historicalGapEnabled = env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_GAP === '1';
  const historicalJournalOnly = env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_JOURNAL_GAP === '1';
  if (historicalJournalOnly && !historicalGapEnabled)
    throw new Error('Journal-only interruption requires the historical gap mode');
  const historicalExportRequested = env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_EXPORT === '1';
  if (historicalExportRequested && (!translationHistoryEnabled || historicalGapEnabled))
    throw new Error('Historical export requires the ordinary SRT history mode');
  const historicalExportEnabled = translationHistoryEnabled && historicalExportRequested;
  if (historicalGapEnabled && !translationHistoryEnabled)
    throw new Error('Historical core gap requires the standalone history mode');
  if (
    translationHistoryEnabled &&
    (!translationEnabled ||
      ['EVENTS', 'CRASH', 'PAUSE', 'PREPARATION_PAUSE', 'DELETE', 'PACKAGE'].some(
        (mode) => env[`AURALIS_NATIVE_E2E_TRANSLATION_${mode}`] === '1',
      ))
  )
    throw new Error('Translation history requires a separate ordinary SRT invocation');
  if (
    translationEventsEnabled &&
    (!translationEnabled ||
      ['VTT', 'CRASH', 'PAUSE', 'PREPARATION_PAUSE', 'DELETE', 'PACKAGE'].some(
        (mode) => env[`AURALIS_NATIVE_E2E_TRANSLATION_${mode}`] === '1',
      ))
  ) {
    throw new Error('Translation events require a separate ordinary SRT invocation');
  }
  const translationVttEnabled =
    translationEnabled && env.AURALIS_NATIVE_E2E_TRANSLATION_VTT === '1';
  const translationCrashEnabled =
    translationEnabled && env.AURALIS_NATIVE_E2E_TRANSLATION_CRASH === '1';
  const translationPauseEnabled =
    translationEnabled && env.AURALIS_NATIVE_E2E_TRANSLATION_PAUSE === '1';
  const translationPreparationPauseEnabled =
    translationEnabled && env.AURALIS_NATIVE_E2E_TRANSLATION_PREPARATION_PAUSE === '1';
  const translationWorkerPreflightPauseEnabled =
    env.AURALIS_NATIVE_E2E_TRANSLATION_WORKER_PREFLIGHT_PAUSE === '1';
  const translationInitialHashPauseEnabled =
    env.AURALIS_NATIVE_E2E_TRANSLATION_INITIAL_HASH_PAUSE === '1';
  const translationStartRaceEnabled = env.AURALIS_NATIVE_E2E_TRANSLATION_START_RACE === '1';
  if (translationStartRaceEnabled && !translationInitialHashPauseEnabled) {
    throw new Error('Concurrent start observation requires the initial-hash preparation mode');
  }
  if (
    translationInitialHashPauseEnabled &&
    (!translationPreparationPauseEnabled || translationWorkerPreflightPauseEnabled)
  ) {
    throw new Error('Initial hash pause requires a separate preparation-pause invocation');
  }
  if (translationWorkerPreflightPauseEnabled && !translationPreparationPauseEnabled) {
    throw new Error('Worker preflight pause requires the preparation-pause mode');
  }
  const translationResultGapEnabled =
    translationCrashEnabled && env.AURALIS_NATIVE_E2E_TRANSLATION_RESULT_GAP === '1';
  const translationDeleteEnabled =
    translationEnabled && env.AURALIS_NATIVE_E2E_TRANSLATION_DELETE === '1';
  const translationPackageEnabled =
    translationEnabled && env.AURALIS_NATIVE_E2E_TRANSLATION_PACKAGE === '1';
  if (
    translationPreparationPauseEnabled &&
    (translationPauseEnabled ||
      translationCrashEnabled ||
      translationDeleteEnabled ||
      translationPackageEnabled)
  ) {
    throw new Error(
      'Native preparation pause runs separately from inference pause, crash, deletion and package modes',
    );
  }
  if (
    translationPauseEnabled &&
    (translationCrashEnabled || translationDeleteEnabled || translationPackageEnabled)
  ) {
    throw new Error(
      'Native pause acceptance does not combine with crash, deletion or package modes',
    );
  }
  if (translationVttEnabled && (translationResultGapEnabled || translationDeleteEnabled)) {
    throw new Error('Native WebVTT acceptance does not combine with result-gap or deletion modes');
  }
  if (
    translationPackageEnabled &&
    (translationVttEnabled || translationCrashEnabled || translationDeleteEnabled)
  ) {
    throw new Error('Native package acceptance runs as a standalone translation mode');
  }
  const secondOwnerEnabled = env.AURALIS_NATIVE_E2E_SECOND_OWNER === '1';
  return {
    translationEnabled,
    casterEnabled,
    translationEventsEnabled,
    translationHistoryEnabled,
    historicalGapEnabled,
    historicalJournalOnly,
    historicalExportEnabled,
    translationVttEnabled,
    translationCrashEnabled,
    translationPauseEnabled,
    translationPreparationPauseEnabled,
    translationWorkerPreflightPauseEnabled,
    translationInitialHashPauseEnabled,
    translationStartRaceEnabled,
    translationResultGapEnabled,
    translationDeleteEnabled,
    translationPackageEnabled,
    secondOwnerEnabled,
  };
}

export function successMessage(env = process.env) {
  return env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_EXPORT === '1'
    ? 'Native Tauri history export E2E passed: exact historical IPC exports, cold retry and offline re-export; sandbox cleaned.\n'
    : env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY === '1'
      ? 'Native Tauri history E2E passed: actual editing, historical preview and selection, retained attachment after restart and immutable outputs; sandbox cleaned.\n'
      : env.AURALIS_NATIVE_E2E_SECOND_OWNER === '1'
        ? 'Native Tauri single-owner E2E passed: a second process was rejected before storage setup, while the first completed its workflow; sandbox cleaned.\n'
        : env.AURALIS_NATIVE_E2E_TRANSLATION_START_RACE === '1'
          ? 'Native concurrent starts E2E passed: parallel requests were busy during observed initial hashing, reused one accepted host job during worker preflight, and preserved pause, fresh resume and the separate ready result; sandbox cleaned.\n'
          : env.AURALIS_NATIVE_E2E_TRANSLATION_INITIAL_HASH_PAUSE === '1'
            ? 'Native initial hash pause E2E passed: actual controls stopped an observed incomplete model hash before any child/job/attempt, then freshly verified and resumed the same run to a separate ready result; sandbox cleaned.\n'
            : env.AURALIS_NATIVE_E2E_TRANSLATION_WORKER_PREFLIGHT_PAUSE === '1'
              ? 'Native worker preflight pause E2E passed: actual controls stopped initial admission and accepted host preflight without a Translate attempt, released both children, and resumed the same run to a separate ready result; sandbox cleaned.\n'
              : env.AURALIS_NATIVE_E2E_TRANSLATION_PREPARATION_PAUSE === '1'
                ? 'Native preparation pause E2E passed: actual controls paused before any job/attempt, released the observed model child and resumed the same run to a separate ready result; sandbox cleaned.\n'
                : env.AURALIS_NATIVE_E2E_TRANSLATION_PAUSE === '1'
                  ? 'Native Tauri pause E2E passed: retained checkpoint and active run, cancelled host job, released managed child and resumed in the same desktop; sandbox cleaned.\n'
                  : env.AURALIS_NATIVE_E2E_TRANSLATION_DELETE === '1'
                    ? 'Native Tauri deletion E2E passed: active checked-model translation stopped and both databases removed the project state; sandbox cleaned.\n'
                    : env.AURALIS_NATIVE_E2E_TRANSLATION_RESULT_GAP === '1'
                      ? 'Native Tauri result-gap E2E passed: a committed Translate result was recovered into a ready separate Auralis artifact after restart; sandbox cleaned.\n'
                      : env.AURALIS_NATIVE_E2E_TRANSLATION_VTT === '1' &&
                          env.AURALIS_NATIVE_E2E_TRANSLATION_CRASH === '1'
                        ? 'Native Tauri WebVTT crash E2E passed: first checkpoint survived desktop termination and the resumed run published a separate copy; sandbox cleaned.\n'
                        : env.AURALIS_NATIVE_E2E_TRANSLATION_CRASH === '1'
                          ? 'Native Tauri crash E2E passed: committed checkpoint survived desktop termination and resumed through a second checked-model attempt; sandbox cleaned.\n'
                          : env.AURALIS_NATIVE_E2E_TRANSLATION_PACKAGE === '1'
                            ? 'Native Tauri package E2E passed: React installed and selected the pinned CPU package, desktop restart restored it, and a real subtitle translation published a separate output; sandbox cleaned.\n'
                            : env.AURALIS_NATIVE_E2E_TRANSLATION_VTT === '1'
                              ? 'Native Tauri WebVTT E2E passed: checked local translation preserved the separate source, protected spans, and both databases; sandbox cleaned.\n'
                              : env.AURALIS_NATIVE_E2E_TRANSLATION === '1'
                                ? 'Native Tauri E2E passed: React -> IPC -> Rust -> checked local translation -> two SQLite files -> separate output; sandbox cleaned.\n'
                                : env.AURALIS_NATIVE_E2E_CASTER === '1'
                                  ? 'Native Caster E2E passed: React catalog controls -> typed IPC -> installed worker -> managed cast and prepared WAV -> restart; sandbox cleaned.\n'
                                  : 'Native Tauri E2E passed: React -> IPC -> Rust -> SQLite -> ffprobe/yt-dlp -> filesystem; sandbox cleaned.\n';
}
