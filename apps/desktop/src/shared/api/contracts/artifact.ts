export type ArtifactKind =
  | 'sourceVideo'
  | 'downloadedVideo'
  | 'extractedAudio'
  | 'originalSubtitle'
  | 'generatedTranscript'
  | 'normalizedTranscript'
  | 'translatedTranscript'
  | 'generatedSpeechSegment'
  | 'mixedAudio'
  | 'previewVideo'
  | 'finalVideo';

export type ArtifactLocation =
  { kind: 'localPath'; value: string } | { kind: 'storageKey'; value: string };

export type ArtifactState = 'pendingFinalize' | 'ready' | 'deleting' | 'failed';

export interface Artifact {
  id: string;
  kind: ArtifactKind;
  location: ArtifactLocation;
  sizeBytes: number | null;
  state: ArtifactState;
  createdAt: string;
  updatedAt: string;
  readyAt: string | null;
}
