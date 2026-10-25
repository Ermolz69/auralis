import { useEffect, useRef, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { invoke } from '@/shared/api/tauri';
import { describeCommandError } from '@/shared/api/contracts/error';
import type {
  AudioAssemblyPreview as AssemblyPreview,
  AssemblySettings,
  AudioAssemblyView as AssemblyView,
} from '@/shared/api/contracts/audioAssembly';
import type { SpeechRunView } from '@/shared/api/contracts/speech';
import type { Artifact } from '@/shared/api/contracts/artifact';

export function useAudioAssembly(projectId: string, source: SpeechRunView | undefined) {
  const [mode, setMode] = useState<AssemblySettings['mode']>('continuous');
  const [duration, setDuration] = useState('');
  const [pauses, setPauses] = useState<Record<number, number>>({});
  const [preview, setPreview] = useState<{ key: string; value: AssemblyPreview } | null>(null);
  const [reviewed, setReviewed] = useState(false);
  const [history, setHistory] = useState<AssemblyView[]>([]);
  const [selected, setSelected] = useState('');
  const [audio, setAudio] = useState<string | null>(null);
  const [video, setVideo] = useState<string | null>(null);
  const [videos, setVideos] = useState<Artifact[]>([]);
  const [videoId, setVideoId] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const alive = useRef(true);
  const durationMs = Number(duration);
  const durationValid = Number.isSafeInteger(durationMs) && durationMs > 0;
  const settings: AssemblySettings = {
    mode,
    source_duration_ms: mode === 'subtitle_window' && durationValid ? durationMs : null,
    pause_after_ms: Array.from({ length: source?.utterance_count ?? 0 }, (_, index) =>
      mode === 'continuous' ? (pauses[index] ?? 0) : 0,
    ),
  };
  const request = {
    source_run_id: source?.run_id ?? '',
    settings,
    ...(videoId ? { video_artifact_id: videoId } : {}),
  };
  const key = JSON.stringify(request);
  const current = preview?.key === key ? preview.value : null;
  const run = history.find((item) => item.assembly_id === selected);
  const active = !!run?.job && ['pending', 'running', 'cancelling'].includes(run.job.status);
  const canInspect = !!source?.result && (mode === 'continuous' || durationValid);

  useEffect(() => {
    alive.current = true;
    void Promise.all([
      invoke('list_project_artifacts_cmd', { projectId, kind: 'downloadedVideo' }),
      invoke('list_project_artifacts_cmd', { projectId, kind: 'sourceVideo' }),
    ])
      .then((items) => {
        if (alive.current) setVideos(items.flat());
      })
      .catch((reason) => {
        if (alive.current) setError(describeCommandError(reason));
      });
    let running = false;
    const refresh = async () => {
      if (running) return;
      running = true;
      try {
        const items = await invoke('list_audio_assemblies_cmd', { projectId });
        if (alive.current) setHistory(items);
      } catch (reason) {
        if (alive.current) setError(describeCommandError(reason));
      } finally {
        running = false;
      }
    };
    void refresh();
    const timer = setInterval(() => void refresh(), 2000);
    return () => {
      alive.current = false;
      clearInterval(timer);
    };
  }, [projectId]);

  const action = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (reason) {
      if (alive.current) setError(describeCommandError(reason));
    } finally {
      if (alive.current) setBusy(false);
    }
  };
  const remember = (result: AssemblyView) => {
    if (!alive.current) return;
    setHistory((items) => [
      result,
      ...items.filter((item) => item.assembly_id !== result.assembly_id),
    ]);
    setSelected(result.assembly_id);
    setAudio(null);
    setVideo(null);
  };
  const changed = () => {
    setReviewed(false);
  };

  return {
    mode,
    duration,
    pauses,
    preview: preview?.value,
    current,
    reviewed,
    history,
    selected,
    run,
    audio,
    video,
    videos,
    videoId,
    setVideoId: (value: string) => {
      changed();
      setVideoId(value);
    },
    busy,
    error,
    active,
    canInspect,
    setMode: (value: AssemblySettings['mode']) => {
      changed();
      setMode(value);
    },
    setDuration: (value: string) => {
      changed();
      setDuration(value);
    },
    setPause: (index: number, value: number) => {
      if (!Number.isSafeInteger(value) || value < 0) return;
      changed();
      setPauses((items) => ({ ...items, [index]: value }));
    },
    setReviewed,
    select: (value: string) => {
      setSelected(value);
      setAudio(null);
      setVideo(null);
    },
    inspect: () =>
      action(async () => {
        if (!canInspect) return;
        const value = await invoke('inspect_audio_assembly_cmd', { projectId, request });
        if (alive.current) {
          setPreview({ key, value });
          setReviewed(false);
        }
      }),
    prepare: () =>
      action(async () => {
        if (!current || !reviewed || current.conflicts.length || !current.renderer_available)
          return;
        remember(
          await invoke('prepare_audio_assembly_cmd', {
            projectId,
            request: {
              source: request,
              expected_preview_sha256: current.preview_sha256,
              reviewed: true,
            },
          }),
        );
      }),
    start: (resume: boolean) =>
      action(async () => {
        if (!run) return;
        remember(
          await invoke(resume ? 'resume_audio_assembly_cmd' : 'start_audio_assembly_cmd', {
            projectId,
            assemblyId: run.assembly_id,
          }),
        );
      }),
    cancel: () =>
      action(async () => {
        if (!run?.job) return;
        await invoke('cancel_job_cmd', { jobId: run.job.id });
        remember(
          await invoke('get_audio_assembly_cmd', { projectId, assemblyId: run.assembly_id }),
        );
      }),
    play: () =>
      action(async () => {
        if (!run?.result) return;
        const verified = await invoke('verify_audio_assembly_cmd', {
          projectId,
          assemblyId: run.assembly_id,
        });
        const file = verified.result?.files.find((item) => item.kind === 'wav');
        if (!file) return;
        const path = await invoke('resolve_artifact_path_cmd', { artifactId: file.artifact_id });
        if (alive.current) setAudio(convertFileSrc(path));
      }),
    playVideo: () =>
      action(async () => {
        setVideo(null);
        if (!run?.result) return;
        const verified = await invoke('verify_audio_assembly_cmd', {
          projectId,
          assemblyId: run.assembly_id,
        });
        const file = verified.result?.files.find((item) => item.kind === 'video');
        if (!file) return;
        const path = await invoke('resolve_artifact_path_cmd', { artifactId: file.artifact_id });
        if (alive.current) setVideo(convertFileSrc(path));
      }),
  };
}
