export type SpeechProfileOption = {
  profile_id: string;
  profile_sha256: string;
  engine_id: string;
};

export type SpeechReferenceOption = {
  reference_id: string;
  engine_id: string;
  emotion_id: string;
  transcript: string;
  audio_sha256: string;
  qualification_status: string;
};

export type SpeechVoiceOption = {
  voice_id: string;
  display_name: string;
  description: string;
  manifest_sha256: string;
  references: SpeechReferenceOption[];
};

export type SpeechCatalogSnapshot = {
  profiles: SpeechProfileOption[];
  voices: SpeechVoiceOption[];
};
