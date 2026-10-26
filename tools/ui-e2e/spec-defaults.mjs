export function inferenceDraft(scope = 'full-source') {
  if (!['diagnostic', 'full-source'].includes(scope))
    throw new Error('SCOPE must be diagnostic or full-source');
  return {
    case_scope: scope,
    allow_inference: false,
    expected_cue_count: 4004,
    source_duration_ms: 4435000,
    source_duration_tolerance_ms: 1000,
    source_timing_sha256: 'f1e967b5113a1ca33499d522f09176845f3ebd142aec2bc413c9f1d471682ee5',
    max_wall_seconds: scope === 'full-source' ? 360000 : 18000,
    tts_execution_budget_seconds: scope === 'full-source' ? 172800 : 2700,
    exercise_cancellation: false,
    export_video: scope === 'full-source',
    install_translation: scope === 'full-source',
    ...(scope === 'diagnostic'
      ? {
          diagnostic_text:
            'Это короткая техническая проверка озвучки. Качество голоса ещё не принято.',
        }
      : {}),
    resource_budget: {
      max_rss_bytes: 24 * 1024 ** 3,
      baseline_vram_bytes: 2048 * 1024 ** 2,
      max_vram_bytes: 7680 * 1024 ** 2,
      min_free_disk_bytes: 64 * 1024 ** 3,
      max_profile_bytes: 64 * 1024 ** 3,
      sample_interval_ms: 1000,
      disk_audit_interval_ms: 30000,
    },
  };
}
