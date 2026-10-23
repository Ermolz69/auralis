import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { invoke } from '@/shared/api/tauri';
import type {
  SpeechAssignment,
  SpeechReview,
  SpeechRunView,
  SpeechSourceSnapshot,
  SpeechSourceRequest,
} from '@/shared/api/contracts/speech';
import { describeCommandError as describe } from '@/shared/api/contracts/error';
import { useSpeechSources } from './useSpeechSources';
import { useMarkerPreparation } from './useMarkerPreparation';
import type { SpeechWorkflowOptions } from './speechOptions';
export type { SpeechWorkflowOptions } from './speechOptions';
import { useSpeechDraftSync } from './useSpeechDraftSync';

export function useSpeechWorkflow({
  projectId,
  profileId,
  voiceId,
  referenceId,
  voices,
  engineId,
  initialSource,
  draft,
  onDraftChange,
}: SpeechWorkflowOptions) {
  const [mode, setMode] = useState<'text' | 'subtitles'>(
    draft?.mode ?? (initialSource && initialSource.kind !== 'text' ? 'subtitles' : 'text'),
  );
  const [text, setText] = useState(draft?.text ?? initialSource?.text ?? '');
  const [translationId, setTranslationId] = useState(
    draft?.translationId ?? initialSource?.translation_id ?? '',
  );
  const [subtitleId, setSubtitleId] = useState(
    draft?.subtitleId ?? initialSource?.artifact_id ?? '',
  );
  const { subtitleFiles, translations, refreshSources } = useSpeechSources(projectId);
  const marker = useMarkerPreparation(projectId, draft);
  const [source, setSource] = useState<SpeechSourceSnapshot | null>(null);
  const [review, setReview] = useState<SpeechReview | null>(null);
  const [assignments, setAssignments] = useState<SpeechAssignment[]>([]);
  const [runs, setRuns] = useState<SpeechRunView[]>([]);
  const [selected, setSelected] = useState(draft?.selected ?? '');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [audio, setAudio] = useState<string | null>(null);
  const run = runs.find((item) => item.run_id === selected);
  const subtitle = subtitleFiles.find((f) => f.id === subtitleId);
  useSpeechDraftSync(
    {
      mode,
      text,
      subtitleId,
      translationId,
      selected,
      preparation: marker.preparation,
      applied: marker.applied,
      originalRequest: marker.originalRequest,
    },
    onDraftChange,
  );
  const request: SpeechSourceRequest = {
    kind: mode === 'text' ? 'text' : translationId ? 'translation' : 'subtitles',
    text: mode === 'text' ? text : null,
    translation_id: translationId || null,
    artifact_id: mode === 'subtitles' && subtitleId ? subtitleId : null,
    format: subtitle
      ? subtitle.location.value.toLowerCase().endsWith('.vtt')
        ? 'vtt'
        : 'srt'
      : null,
  };

  useEffect(() => {
    let active = true;
    setSource(null);
    setReview(null);
    setAssignments([]);
    setSelected('');
    setAudio(null);
    setError(null);
    const refresh = () => {
      void invoke('list_speech_runs_cmd', { projectId })
        .then((items) => {
          if (active) setRuns(items);
        })
        .catch((reason: unknown) => {
          if (active) setError(describe(reason));
        });
    };
    refresh();
    const timer = setInterval(refresh, 2000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [projectId]);

  useEffect(() => {
    setAudio(null);
  }, [selected]);
  useEffect(() => {
    setAssignments([]);
    setSelected('');
  }, [profileId]);

  const action = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (reason) {
      setError(describe(reason));
    } finally {
      setBusy(false);
    }
  };
  const changed = () => {
    marker.clear();
    setSource(null);
    setReview(null);
    setAssignments([]);
    setSelected('');
  };
  const load = () =>
    action(async () => {
      setSource(await invoke('get_speech_source_cmd', { projectId, source: request }));
      setReview(null);
    });
  const confirm = () =>
    action(async () => {
      const result = marker.applied
        ? await marker.confirm()
        : await invoke('review_speech_text_cmd', {
            projectId,
            source: request,
            confirmed: true,
          });
      setReview(result);
      setSource(result.source);
    });
  const prepare = () =>
    action(async () => {
      if (!review) return;
      const result = await invoke('prepare_speech_plan_cmd', {
        projectId,
        request: {
          decision_id: review.decision_id,
          expected_source_sha256: review.source.sha256,
          profile_id: profileId,
          voice_id: voiceId,
          reference_id: referenceId,
          assignments,
        },
      });
      setRuns((items) => [result, ...items.filter((item) => item.run_id !== result.run_id)]);
      setSelected(result.run_id);
    });
  const start = (resume: boolean) =>
    action(async () => {
      if (!run) return;
      const result = await invoke(resume ? 'resume_speech_run_cmd' : 'start_speech_run_cmd', {
        projectId,
        runId: run.run_id,
      });
      setRuns((items) => items.map((item) => (item.run_id === result.run_id ? result : item)));
    });
  const play = () =>
    action(async () => {
      const file = run?.result?.files.find((item) => item.kind === 'wav');
      if (!file) return;
      const path = await invoke('resolve_artifact_path_cmd', { artifactId: file.artifact_id });
      setAudio(convertFileSrc(path));
    });
  const assign = (scope: 'role' | 'cue', target: string, assignedVoice: string) => {
    const reference = voices
      .find((v) => v.voice_id === assignedVoice)
      ?.references.find((r) => r.engine_id === engineId && r.emotion_id === 'neutral');
    setAssignments((items) => [
      ...items.filter((a) => a.scope !== scope || a.target !== target),
      ...(reference
        ? [{ scope, target, voice_id: assignedVoice, reference_id: reference.reference_id }]
        : []),
    ]);
  };
  const active = run?.job && ['pending', 'running', 'cancelling'].includes(run.job.status);

  return {
    markerPreparation: marker.preparation,
    markerApplied: marker.applied,
    normalize: () =>
      action(async () => {
        await marker.normalize(request);
        setReview(null);
        setSelected('');
      }),
    applyPreparation: () => {
      marker.apply();
      if (marker.preparation) {
        setSource(marker.preparation.prepared);
        if (mode === 'text') setText(marker.preparation.prepared.text);
      }
      setReview(null);
      setSelected('');
    },
    mode,
    setMode,
    text,
    setText,
    translationId,
    setTranslationId,
    subtitleId,
    setSubtitleId,
    subtitleFiles,
    translations,
    source,
    review,
    assignments,
    runs,
    selected,
    setSelected,
    error,
    busy,
    audio,
    run,
    active,
    changed,
    load,
    confirm,
    prepare,
    start,
    play,
    assign,
    refreshSources: () => action(refreshSources),
    cancel: () =>
      action(async () => {
        if (run?.job) await invoke('cancel_job_cmd', { jobId: run.job.id });
      }),
  };
}
