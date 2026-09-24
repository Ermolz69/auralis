export interface TranslationSegment {
  id: number;
  ordinal: number;
  cueLabel: string | null;
  startMs: number;
  endMs: number;
  sourceLines: string[];
  textRanges: [number, number][];
}

export interface TranslationInspection {
  format: 'srt' | 'vtt';
  sourceSha256: string;
  sourceSize: number;
  parserVersion: number;
  segments: TranslationSegment[];
}
