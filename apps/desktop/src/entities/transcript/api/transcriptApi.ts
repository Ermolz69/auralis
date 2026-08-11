import { invoke } from '@/shared/api/tauri';
import type { Transcript } from '../model/types';
import type { SubtitleTrack } from '@/shared/api/contracts/subtitle';

export async function getTranscript(projectId: string): Promise<Transcript | null> {
  return invoke('get_transcript_cmd', { projectId });
}

export async function listYoutubeSubtitleTracks(projectId: string): Promise<SubtitleTrack[]> {
  return invoke('list_youtube_subtitle_tracks_cmd', { projectId });
}
