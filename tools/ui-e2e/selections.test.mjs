import assert from 'node:assert/strict';
import { test } from 'node:test';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { selectionIdentities, verifySelectionPins } from './selections.mjs';
import { hash, owned } from './core.mjs';
test('installed selection metadata must match the frozen voice, model and reference bytes', async () => {
  const profile = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-pins-'));
  try {
    const profiles = path.join(profile, 'profiles'),
      voices = path.join(profile, 'voices');
    await fsp.mkdir(profiles);
    await fsp.mkdir(path.join(voices, 'voice'), { recursive: true });
    const audio = path.join(voices, 'voice', 'sample.wav');
    await fsp.writeFile(audio, 'reference identity fixture');
    await fsp.writeFile(
      path.join(profile, 'tts-runtime.json'),
      JSON.stringify({ profile_root: profiles, voice_store: voices }),
    );
    await fsp.writeFile(
      path.join(profiles, 'profile.json'),
      JSON.stringify({ engine: 'voxcpm2', model_revision: 'a'.repeat(40) }),
    );
    await fsp.writeFile(
      path.join(voices, 'voice', 'voice.json'),
      JSON.stringify({
        references: [
          {
            reference_id: 'ref',
            engine_id: 'voxcpm2',
            audio: { path: 'sample.wav', sha256: await hash(audio) },
          },
        ],
      }),
    );
    const selection = {
      engine_id: 'voxcpm2',
      profile_id: 'profile',
      voice_id: 'voice',
      reference_id: 'ref',
    };
    const pins = await selectionIdentities(profile, [selection]);
    verifySelectionPins(await selectionIdentities(profile, [selection]), pins);
    assert.throws(() =>
      verifySelectionPins(pins, [{ ...pins[0], model_revision: 'b'.repeat(40) }]),
    );
    await fsp.writeFile(audio, 'changed reference');
    await assert.rejects(selectionIdentities(profile, [selection]));
  } finally {
    await fsp.rm(owned(os.tmpdir(), profile), { recursive: true, force: true });
  }
});
