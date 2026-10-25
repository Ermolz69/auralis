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
import type { SpeechVoiceOption } from '@/shared/api/contracts/speechCatalog';
import { describeCommandError } from '@/shared/api/contracts/error';
import { useSpeechSources } from './useSpeechSources';
import { useMarkerPreparation } from './useMarkerPreparation';
import { referenceAllowed } from './referenceAllowed';

export type SpeechWorkflowOptions = {
  projectId: string;
  profileId: string;
  voiceId: string;
  referenceId: string;
  voices: SpeechVoiceOption[];
  engineId: string;
  allowUnverifiedNeutral?: boolean;
};
const describe = describeCommandError;

export function useSpeechWorkflow({
  projectId,
  profileId,
  voiceId,
  referenceId,
  voices,
  engineId,
  allowUnverifiedNeutral = false,
}: SpeechWorkflowOptions) {
  const [mode, setMode] = useState<'text' | 'subtitles'>('text');
  const [text, setText] = useState('');
  const [translationId, setTranslationId] = useState('');
  const [subtitleId, setSubtitleId] = useState('');
  const { subtitleFiles, translations, refreshSources } = useSpeechSources(projectId);
  const marker = useMarkerPreparation(projectId);
  const [source, setSource] = useState<SpeechSourceSnapshot | null>(null);
  const [review, setReview] = useState<SpeechReview | null>(null);
  const [assignments, setAssignments] = useState<SpeechAssignment[]>([]);
  const [runs, setRuns] = useState<SpeechRunView[]>([]);
  const [selected, setSelected] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [audio, setAudio] = useState<string | null>(null);
  const run = runs.find((item) => item.run_id === selected);
  const subtitle = subtitleFiles.find((f) => f.id === subtitleId);
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
  }, [profileId, allowUnverifiedNeutral]);

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
          ...(allowUnverifiedNeutral ? { allow_unverified_neutral: true } : {}),
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
  const assign = (
    scope: 'role' | 'cue',
    target: string,
    assignedVoice: string,
    assignedReference?: string,
  ) => {
    const preferredEmotion = voices
      .find((v) => v.voice_id === voiceId)
      ?.references.find((r) => r.reference_id === referenceId)?.emotion_id;
    const compatible = voices
      .find((v) => v.voice_id === assignedVoice)
      ?.references.filter(
        (r) => r.engine_id === engineId && referenceAllowed(r, allowUnverifiedNeutral),
      );
    const reference = assignedReference
      ? compatible?.find((r) => r.reference_id === assignedReference)
      : (compatible?.find((r) => r.emotion_id === preferredEmotion) ??
        compatible?.find((r) => r.emotion_id === 'neutral') ??
        compatible?.[0]);
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
