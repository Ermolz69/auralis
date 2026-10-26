import path from 'node:path';
import fsp from 'node:fs/promises';
import assert from 'node:assert/strict';
import { hash, json, owned } from './core.mjs';

export async function selectionIdentities(profile, engines, remaining = () => Infinity) {
  const config = await json(path.join(profile, 'tts-runtime.json'));
  const facts = [];
  for (const selection of engines) {
    remaining();
    const profileFile = owned(
      profile,
      path.join(config.profile_root, `${selection.profile_id}.json`),
    );
    const voiceFile = owned(
      profile,
      path.join(config.voice_store, selection.voice_id, 'voice.json'),
    );
    for (const file of [profileFile, voiceFile]) assert.equal(await fsp.realpath(file), file);
    const settings = await json(profileFile),
      voice = await json(voiceFile);
    assert.equal(settings.engine, selection.engine_id);
    const reference = voice.references.find(
      (r) => r.reference_id === selection.reference_id && r.engine_id === selection.engine_id,
    );
    assert.ok(reference, 'Frozen reference is missing');
    const audio = owned(
      path.dirname(voiceFile),
      path.join(path.dirname(voiceFile), reference.audio.path),
    );
    assert.equal(await fsp.realpath(audio), audio);
    assert.equal(await hash(audio, remaining), reference.audio.sha256);
    facts.push({
      ...selection,
      profile_sha256: await hash(profileFile, remaining),
      voice_manifest_sha256: await hash(voiceFile, remaining),
      reference_audio_sha256: reference.audio.sha256,
      model_revision: settings.model_revision,
    });
  }
  return facts;
}

export function verifySelectionPins(actual, expected) {
  assert.deepEqual(
    actual,
    expected,
    'Installed profile, model revision, voice or reference differs from the frozen case',
  );
}
