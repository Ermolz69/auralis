import { invoke } from '@/shared/api/tauri';
import type { Transcript } from '../model/types';

export async function getTranscript(projectId: string): Promise<Transcript | null> {
  return invoke<Transcript | null>('get_transcript_cmd', { projectId });
}
