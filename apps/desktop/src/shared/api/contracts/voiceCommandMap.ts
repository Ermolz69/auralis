import type {
  CastResult,
  LogicalCast,
  ManualSelection,
  ReferencePlayback,
  VoiceCatalogPage,
  VoiceCompatibility,
  VoiceDetails,
} from './caster';
import type { SpeechCatalogSnapshot } from './speechCatalog';

export interface VoiceCommandMap {
  get_speech_catalog_cmd: { args: { projectId: string }; result: SpeechCatalogSnapshot };
  list_voice_catalog_cmd: {
    args: { projectId: string; after?: string | null; limit: number };
    result: VoiceCatalogPage;
  };
  inspect_voice_cmd: {
    args: { projectId: string; voiceId: string; afterReference?: string | null; limit: number };
    result: VoiceDetails;
  };
  inspect_voice_reference_cmd: {
    args: { projectId: string; voiceId: string; referenceId: string };
    result: ReferencePlayback;
  };
  check_voice_compatibility_cmd: {
    args: { projectId: string; voiceId: string; engineId: string; emotionId: string };
    result: VoiceCompatibility;
  };
  get_manual_cast_cmd: { args: { projectId: string }; result: CastResult | null };
  assign_manual_cast_cmd: {
    args: {
      projectId: string;
      cast: LogicalCast;
      selections: Record<string, ManualSelection>;
      expectedRevision?: string | null;
      requestId: string;
    };
    result: CastResult;
  };
  recover_manual_cast_cmd: { args: { projectId: string }; result: string };
}
