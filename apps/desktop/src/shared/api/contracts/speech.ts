import type { Job } from './job';

export type SpeechSourceRequest = {
  kind: 'text' | 'subtitles' | 'translation';
  text: string | null;
  translation_id: string | null;
  artifact_id?: string | null;
  format?: 'srt' | 'vtt' | null;
};
export type SpeechCue = {
  id: string;
  text: string;
  role: string;
  start_ms: number;
  end_ms: number;
};
export type SpeechSourceSnapshot = {
  kind: string;
  source_id: string | null;
  revision: string;
  sha256: string;
  text: string;
  cues: SpeechCue[];
};
export type SpeechReview = {
  original_source: SpeechSourceSnapshot | null;
  decision_id: string;
  project_id: string;
  source: SpeechSourceSnapshot;
};
export type SpeechAssignment = {
  scope: 'role' | 'cue';
  target: string;
  voice_id: string;
  reference_id: string;
};
export type PrepareSpeechRequest = {
  decision_id: string;
  expected_source_sha256: string;
  profile_id: string;
  voice_id: string;
  reference_id: string;
  assignments: SpeechAssignment[];
};
export type SpeechPublication = {
  run_id: string;
  attempt_id: string;
  duration_ms: number;
  ready: boolean;
  files: { artifact_id: string; kind: string; sha256: string; size_bytes: number }[];
};
export type SpeechRunView = {
  run_id: string;
  profile_id: string;
  utterance_count: number;
  chunk_count: number;
  job: Job | null;
  result: SpeechPublication | null;
};

export interface SpeechCommandMap {
  get_speech_voice_selection_cmd: { args: { projectId: string }; result: PuzaVoiceBinding | null };
  save_speech_voice_selection_cmd: {
    args: { projectId: string; profileId: string; voiceId: string; referenceId: string };
    result: PuzaVoiceBinding;
  };
  prepare_speech_text_cmd: {
    args: { projectId: string; source: SpeechSourceRequest };
    result: MarkerPreparation;
  };
  review_prepared_speech_text_cmd: {
    args: {
      projectId: string;
      source: SpeechSourceRequest;
      preparation: MarkerPreparation;
      confirmed: boolean;
    };
    result: SpeechReview;
  };
  get_speech_source_cmd: {
    args: { projectId: string; source: SpeechSourceRequest };
    result: SpeechSourceSnapshot;
  };
  review_speech_text_cmd: {
    args: { projectId: string; source: SpeechSourceRequest; confirmed: boolean };
    result: SpeechReview;
  };
  prepare_speech_plan_cmd: {
    args: { projectId: string; request: PrepareSpeechRequest; assetId?: string | null };
    result: SpeechRunView;
  };
  start_speech_run_cmd: { args: { projectId: string; runId: string }; result: SpeechRunView };
  resume_speech_run_cmd: { args: { projectId: string; runId: string }; result: SpeechRunView };
  get_speech_run_cmd: { args: { projectId: string; runId: string }; result: SpeechRunView };
  list_speech_runs_cmd: { args: { projectId: string }; result: SpeechRunView[] };
}

export type MarkerPreparation = {
  version: string;
  original: SpeechSourceSnapshot;
  prepared: SpeechSourceSnapshot;
  changed_targets: number;
  unresolved_targets: number;
};
export type PuzaVoiceBinding = {
  role_id: string;
  display_name: string;
  voice_id: string;
  reference_id: string;
  engine_id: string;
};
