export function installUiSpeechFixture() {
  const base = window.__TAURI_INTERNALS__.invoke;
  const state = window.__ui;
  const digest = 'a'.repeat(64);
  const timestamp = '2026-10-24T00:00:00Z';
  state.speeches ??= {};
  state.assemblies ??= {};
  const source = (text) => ({
    kind: 'text',
    source_id: null,
    revision: digest,
    sha256: digest,
    text,
    cues: [],
  });
  const job = (kind, status) => ({
    id: `fixture-${kind}-${kind === 'speech' ? state.speech.run_id : state.assembly.assembly_id}`,
    kind,
    revision: 1,
    projectId: '00000000-0000-4000-8000-000000000001',
    title: 'Fixture operation',
    status,
    stage: 'postprocessAudio',
    progress: {
      percent: status === 'completed' ? 100 : 35,
      message: 'Fixture operation',
      currentStep: null,
      processedItems: null,
      totalItems: null,
    },
    error: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  });
  const file = (kind, owner) => ({
    artifact_id: `fixture-${kind}-${owner}`,
    kind,
    sha256: digest,
    size_bytes: 480044,
  });
  const videoSource = (id) =>
    id
      ? {
          policy_id: 'replace_original_audio_pad_silence_h264_aac_v1',
          artifact: { artifact_id: id, sha256: digest, size_bytes: 100 },
          media: {
            sha256: digest,
            size_bytes: 100,
            duration_ms: 5000,
            width: 320,
            height: 180,
            video_codec: 'fixture-webm',
            ffprobe_sha256: digest,
          },
        }
      : null;
  const finishSpeech = () => {
    state.speech = {
      ...state.speech,
      job: job('speech', 'completed'),
      result: {
        run_id: state.speech.run_id,
        attempt_id: `fixture-attempt-${state.speech.run_id}`,
        duration_ms: 5000,
        ready: true,
        files: ['wav', 'manifest', 'segment', 'audio_layout'].map(kind => file(kind, state.speech.run_id)),
      },
    };
  };
  const finishAssembly = () => {
    state.assembly = {
      ...state.assembly,
      job: job('audioAssembly', 'completed'),
      result: {
        assembly_id: state.assembly.assembly_id,
        attempt_id: `fixture-attempt-${state.assembly.assembly_id}`,
        sample_rate: 48000,
        channels: 1,
        frames: 240000,
        pcm_sha256: digest,
        peak_sample_abs: 0,
        full_scale_samples: 0,
        ready: true,
        files: (state.assembly.video
          ? ['wav', 'manifest', 'video', 'video_manifest']
          : ['wav', 'manifest']
        ).map(kind => file(kind, state.assembly.assembly_id)),
      },
    };
  };
  const wav = new Uint8Array(480044);
  const view = new DataView(wav.buffer);
  const ascii = (offset, text) => [...text].forEach((c, i) => (wav[offset + i] = c.charCodeAt(0)));
  ascii(0, 'RIFF');
  view.setUint32(4, 480036, true);
  ascii(8, 'WAVEfmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, 48000, true);
  view.setUint32(28, 96000, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  ascii(36, 'data');
  view.setUint32(40, 480000, true);
  const audio = URL.createObjectURL(new Blob([wav], { type: 'audio/wav' }));
  const convert = window.__TAURI_INTERNALS__.convertFileSrc;
  window.__TAURI_INTERNALS__.convertFileSrc = (file, ...args) =>
    file === 'fixture.wav' ? audio : convert(file, ...args);
  window.__TAURI_INTERNALS__.invoke = async (command, args = {}) => {
    let value;
    if (command === 'get_speech_catalog_cmd')
      value = {
        profiles: ['voxcpm2', 'espeech'].map((engine) => ({
          profile_id: `native-plan-${engine}`,
          engine_id: engine,
          profile_sha256: digest,
        })),
        voices: [
          {
            voice_id: 'fixture-speaker',
            display_name: 'Fixture speaker',
            description: 'Synthetic test data',
            manifest_sha256: digest,
            references: ['voxcpm2', 'espeech'].map((engine) => ({
              reference_id: `fixture-${engine}`,
              engine_id: engine,
              emotion_id: 'neutral',
              transcript: 'Fixture reference.',
              audio_sha256: digest,
              qualification_status: 'unverified',
            })),
          },
        ],
      };
    else if (command === 'get_speech_voice_selection_cmd') value = state.voice ?? null;
    else if (command === 'save_speech_voice_selection_cmd')
      value = state.voice = {
        role_id: 'narrator',
        display_name: 'Narrator',
        voice_id: args.voiceId,
        reference_id: args.referenceId,
        engine_id: args.profileId.endsWith('espeech') ? 'espeech' : 'voxcpm2',
      };
    else if (command === 'get_speech_source_cmd')
      value = source(args.source.text ?? state.edited ?? 'Здравствуйте.');
    else if (command === 'review_speech_text_cmd') {
      if (args.source.kind === 'translation' && !state.review)
        throw { code: 'CONFLICT', message: 'Fixture translation approval is required' };
      value = {
        decision_id: 'fixture-review',
        project_id: args.projectId,
        original_source: null,
        source: source(args.source.text ?? state.edited ?? 'Здравствуйте.'),
      };
    } else if (command === 'prepare_speech_plan_cmd') {
      if (!args.request.allow_unverified_neutral)
        throw {
          code: 'VALIDATION',
          message: 'Fixture neutral reference requires explicit technical permission',
        };
      if (state.result && !state.review)
        throw { code: 'CONFLICT', message: 'Fixture translation approval is stale' };
      value = state.speech = {
        run_id: `fixture-raw-${args.request.profile_id.replace('native-plan-', '')}`,
        technical_reference_permission: 'fixture-technical-permission',
        profile_id: args.request.profile_id,
        utterance_count: 1,
        chunk_count: 1,
        job: null,
        result: null,
      };
    } else if (command === 'list_speech_runs_cmd') value = Object.values(state.speeches);
    else if (command === 'get_speech_run_cmd') value = state.speeches[args.runId];
    else if (command === 'start_speech_run_cmd') {
      state.speech = state.speeches[args.runId];
      value = state.speech = { ...state.speech, job: job('speech', 'running') };
    } else if (command === 'resume_speech_run_cmd') {
      state.speech = state.speeches[args.runId];
      finishSpeech();
      value = state.speech;
    } else if (command === 'cancel_job_cmd') {
      if (args.jobId.startsWith('fixture-audioAssembly-')) {
        state.assembly.job = job('audioAssembly', 'cancelled');
        value = state.assembly.job;
      } else {
        state.speech.job = job('speech', 'cancelled');
        value = state.speech.job;
      }
    } else if (command === 'resolve_artifact_path_cmd') value = 'fixture.wav';
    else if (command === 'list_audio_assemblies_cmd') value = Object.values(state.assemblies);
    else if (command === 'inspect_audio_assembly_cmd')
      value = {
        video: videoSource(args.request.video_artifact_id),
        source_run_id: args.request.source_run_id,
        source_plan_sha256: digest,
        preview_sha256: digest,
        profile_id: state.speech.profile_id,
        settings: args.request.settings,
        sample_rate: 48000,
        channels: 1,
        frames: args.request.settings.mode === 'continuous' ? 240000 : null,
        utterances: [
          {
            utterance_id: 'fixture-utterance',
            audio_sha256: digest,
            sample_rate: 48000,
            channels: 1,
            frame_count: 240000,
            source_window: { start_ms: 0, end_ms: 5000 },
            pause_after_ms: args.request.settings.pause_after_ms[0],
          },
        ],
        conflicts:
          args.request.settings.mode === 'continuous'
            ? []
            : [
                {
                  kind: 'impossible_fit',
                  utterance_id: 'fixture-utterance',
                  required_frames: 240000,
                  available_frames: 48,
                },
              ],
        renderer_available: true,
        renderer_error: null,
      };
    else if (command === 'prepare_audio_assembly_cmd')
      value = state.assembly = {
        video: videoSource(args.request.source.video_artifact_id),
        assembly_id: `fixture-assembly-${Object.keys(state.assemblies).length + 1}`,
        source_run_id: args.request.source.source_run_id,
        plan_sha256: digest,
        settings: args.request.source.settings,
        utterance_count: 1,
        frames: 240000,
        job: null,
        result: null,
      };
    else if (command === 'start_audio_assembly_cmd') {
      state.assembly = state.assemblies[args.assemblyId];
      if (!state.assembly.video) state.assemblyStarts = (state.assemblyStarts ?? 0) + 1;
      state.assembly = { ...state.assembly, job: job('audioAssembly', 'running') };
      if (state.assembly.video || state.assemblyStarts <= 2) finishAssembly();
      value = state.assembly;
    } else if (command === 'resume_audio_assembly_cmd') {
      state.assembly = state.assemblies[args.assemblyId];
      finishAssembly();
      value = state.assembly;
    } else if (command === 'get_audio_assembly_cmd' || command === 'verify_audio_assembly_cmd') {
      if (command === 'verify_audio_assembly_cmd' && state.verifyFailure)
        throw { code: 'VALIDATION', message: 'Fixture integrity verification failed' };
      value = state.assemblies[args.assemblyId];
    } else return base(command, args);
    state.calls.push({ command, args });
    if (state.speech) state.speeches[state.speech.run_id] = state.speech;
    if (state.assembly) state.assemblies[state.assembly.assembly_id] = state.assembly;
    sessionStorage.setItem('ui-fixture', JSON.stringify(state));
    return structuredClone(value);
  };
}
