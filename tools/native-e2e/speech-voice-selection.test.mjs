import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { chooseQualifiedSpeechVoices, readSpeechVoices } from './speech.mjs';

function manifest(voiceId, qualifiedEngines) {
  return {
    voice_id: voiceId,
    references: ['voxcpm2', 'espeech'].map((engine) => ({
      engine_id: engine,
      emotion_id: 'neutral',
      qualification: {
        status: qualifiedEngines.includes(engine) ? 'qualified' : 'unverified',
        report: qualifiedEngines.includes(engine)
          ? { path: 'review.json', sha256: 'a'.repeat(64) }
          : null,
      },
    })),
  };
}

test('native matrix chooses current qualified voices without relying on Marina or Alex', () => {
  const manifests = new Map([
    ['marina_elevenlabs_ru', manifest('marina_elevenlabs_ru', [])],
    ['alex_analyst_elevenlabs_ru', manifest('alex_analyst_elevenlabs_ru', ['voxcpm2'])],
    [
      'anastasiya_kim_elevenlabs_ru',
      manifest('anastasiya_kim_elevenlabs_ru', ['voxcpm2', 'espeech']),
    ],
    ['rina_elevenlabs_ru', manifest('rina_elevenlabs_ru', ['voxcpm2', 'espeech'])],
    ['ciri', manifest('ciri', ['voxcpm2', 'espeech'])],
  ]);
  assert.deepEqual(chooseQualifiedSpeechVoices(manifests), [
    'anastasiya_kim_elevenlabs_ru',
    'rina_elevenlabs_ru',
  ]);
});

test('native matrix refuses incomplete human qualification', () => {
  const manifests = new Map([
    ['marina_elevenlabs_ru', manifest('marina_elevenlabs_ru', ['voxcpm2', 'espeech'])],
    ['rina_elevenlabs_ru', manifest('rina_elevenlabs_ru', ['voxcpm2'])],
  ]);
  assert.throws(() => chooseQualifiedSpeechVoices(manifests), /Human qualification missing/);
});

test('retained voice selection detects a changed manifest before restarting the app', async () => {
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'auralis-speech-voices-'));
  try {
    const voiceStore = path.join(temporary, 'voices');
    const voices = ['anastasiya_kim_elevenlabs_ru', 'rina_elevenlabs_ru'];
    const hashes = {};
    for (const voice of voices) {
      const folder = path.join(voiceStore, voice);
      await fs.mkdir(folder, { recursive: true });
      const bytes = Buffer.from(JSON.stringify(manifest(voice, ['voxcpm2', 'espeech'])));
      await fs.writeFile(path.join(folder, 'voice.json'), bytes);
      hashes[voice] = crypto.createHash('sha256').update(bytes).digest('hex');
    }
    await fs.writeFile(
      path.join(temporary, 'tts-runtime.json'),
      JSON.stringify({ voice_store: voiceStore }),
    );
    await fs.writeFile(
      path.join(temporary, 'speech-e2e-voices.json'),
      JSON.stringify({ version: 1, voice_ids: voices, manifest_sha256: hashes }),
    );
    assert.deepEqual(await readSpeechVoices(temporary), voices);
    await fs.appendFile(path.join(voiceStore, voices[0], 'voice.json'), '\n');
    await assert.rejects(readSpeechVoices(temporary), /changed/);
  } finally {
    const resolved = await fs.realpath(temporary);
    const tempRoot = await fs.realpath(os.tmpdir());
    if (path.dirname(resolved) !== tempRoot)
      throw new Error('Refusing to remove a voice fixture outside TEMP');
    await fs.rm(resolved, { recursive: true, force: true });
  }
});
