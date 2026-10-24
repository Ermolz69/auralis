import type { Job } from './job';

export type AssemblySettings = {
  mode: 'continuous' | 'subtitle_window';
  source_duration_ms: number | null;
  pause_after_ms: number[];
};
export type InspectAssemblyRequest = { source_run_id: string; settings: AssemblySettings };
export type PrepareAssemblyRequest = {
  source: InspectAssemblyRequest;
  expected_preview_sha256: string;
  reviewed: boolean;
};
export type MeasuredSpeech = {
  utterance_id: string;
  audio_sha256: string;
  sample_rate: number;
  channels: number;
  frame_count: number;
  source_window: { start_ms: number; end_ms: number } | null;
  pause_after_ms: number;
};
export type TimingConflict =
  | { kind: 'invalid_count'; actual: number; max: number }
  | { kind: 'coverage_mismatch' | 'missing_media_duration' | 'arithmetic_overflow' }
  | {
      kind: 'duplicate_utterance' | 'invalid_audio' | 'missing_window' | 'pause_in_subtitle_window';
      utterance_id: string;
    }
  | { kind: 'channel_mismatch'; utterance_id: string; expected: number; actual: number }
  | { kind: 'invalid_window'; utterance_id: string; start_ms: number; end_ms: number }
  | { kind: 'outside_media'; utterance_id: string; end_ms: number; duration_ms: number }
  | { kind: 'overlapping_windows'; utterance_id: string; previous_utterance_id: string }
  | {
      kind: 'impossible_fit';
      utterance_id: string;
      required_frames: number;
      available_frames: number;
    }
  | { kind: 'riff_size_limit'; frames: number; channels: number };
export type AudioAssemblyPreview = {
  source_run_id: string;
  source_plan_sha256: string;
  preview_sha256: string;
  profile_id: string;
  settings: AssemblySettings;
  sample_rate: number;
  channels: number;
  frames: number | null;
  utterances: MeasuredSpeech[];
  conflicts: TimingConflict[];
  renderer_available: boolean;
  renderer_error: string | null;
};
export type AssemblyPublication = {
  assembly_id: string;
  attempt_id: string;
  sample_rate: number;
  channels: number;
  frames: number;
  pcm_sha256: string;
  peak_sample_abs: number;
  full_scale_samples: number;
  files: { artifact_id: string; kind: 'wav' | 'manifest'; sha256: string; size_bytes: number }[];
  ready: boolean;
};
export type AudioAssemblyView = {
  assembly_id: string;
  source_run_id: string;
  plan_sha256: string;
  settings: AssemblySettings;
  utterance_count: number;
  frames: number;
  job: Job | null;
  result: AssemblyPublication | null;
};
export interface AudioAssemblyCommandMap {
  inspect_audio_assembly_cmd: {
    args: { projectId: string; request: InspectAssemblyRequest };
    result: AudioAssemblyPreview;
  };
  prepare_audio_assembly_cmd: {
    args: { projectId: string; request: PrepareAssemblyRequest };
    result: AudioAssemblyView;
  };
  start_audio_assembly_cmd: {
    args: { projectId: string; assemblyId: string };
    result: AudioAssemblyView;
  };
  resume_audio_assembly_cmd: {
    args: { projectId: string; assemblyId: string };
    result: AudioAssemblyView;
  };
  get_audio_assembly_cmd: {
    args: { projectId: string; assemblyId: string };
    result: AudioAssemblyView;
  };
  verify_audio_assembly_cmd: {
    args: { projectId: string; assemblyId: string };
    result: AudioAssemblyView;
  };
  list_audio_assemblies_cmd: { args: { projectId: string }; result: AudioAssemblyView[] };
}
