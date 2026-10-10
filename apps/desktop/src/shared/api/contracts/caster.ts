export type VoiceCapability = { engine_id: string; emotion_id: string };

export type VoiceCatalogEntry = {
  voice_id: string;
  state: 'valid' | 'legacy' | 'invalid';
  display_name: string | null;
  description: string | null;
  manifest_sha256: string | null;
  reference_count: number | null;
  qualified_capabilities: VoiceCapability[] | null;
  error_code: string | null;
};

export type VoiceCatalogPage = { items: VoiceCatalogEntry[]; next_after: string | null };

export type VoiceReferenceMetadata = {
  reference_id: string;
  engine_id: string;
  emotion_id: string;
  transcript: string;
  audio_sha256: string;
  qualification_status: 'unverified' | 'qualified' | 'rejected';
  qualification_report_sha256: string | null;
};

export type VoiceDetails = {
  voice_id: string;
  display_name: string;
  description: string;
  manifest_sha256: string;
  references: VoiceReferenceMetadata[];
  next_after_reference: string | null;
};

export type ReferencePlayback = VoiceReferenceMetadata & {
  voice_id: string;
  playback_path: string;
};

export type VoiceCompatibility = VoiceCapability & {
  voice_id: string;
  compatible: boolean;
  reference_ids: string[];
};

export type CastRole = { display_name: string; voice_id: string | null };
export type LogicalCast = { version: 2; roles: Record<string, CastRole> };
export type ManualSelection = VoiceCapability & { voice_id: string; reference_id: string };
export type CastSelectionSnapshot = ManualSelection & {
  manifest_sha256: string;
  audio_sha256: string;
  report_sha256: string;
  transcript: string;
};
export type CastRevision = {
  version: 1;
  parent_revision: string | null;
  request_id: string;
  request_sha256: string;
  cast: LogicalCast;
  selections: Record<string, CastSelectionSnapshot>;
};
export type CastResult = { revision_sha256: string; revision: CastRevision };
