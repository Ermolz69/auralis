export interface TranscriptSegment {
  id: string;
  index: number;
  startMs: number;
  endMs: number;
  sourceText: string;
}

export interface Transcript {
  language: string;
  segments: TranscriptSegment[];
}
