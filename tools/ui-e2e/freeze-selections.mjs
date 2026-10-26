import path from 'node:path';
import fsp from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import assert from 'node:assert/strict';
import { hash, json, root, storage, owned } from './core.mjs';

// Read metadata with Python's standard library only. No component/engine imports.
const wheelProfiles = `import hashlib,json,sys,zipfile
with zipfile.ZipFile(sys.argv[1]) as wheel:
 result={}
 for profile in json.loads(sys.argv[2]):
  names=[n for n in wheel.namelist() if n.endswith('share/puza-tts/profiles/'+profile+'.json')]
  if len(names)!=1: raise ValueError('Expected one pinned wheel profile')
  data=wheel.read(names[0])
  result[profile]={'sha256':hashlib.sha256(data).hexdigest(),'settings':json.loads(data)}
 print(json.dumps(result))`;

export async function freezeSelections(seed, wheels, engines) {
  const directory = owned(path.join(storage, 'seeds'), path.resolve(root, seed.directory));
  const verified = async (file) => {
    const relative = path.relative(directory, owned(directory, file)).replaceAll('\\', '/');
    const item = seed.files.find((item) => item.path.replaceAll('\\', '/') === relative);
    assert.ok(item, `Selection metadata is outside the pinned seed: ${relative}`);
    assert.equal(await fsp.realpath(file), file);
    assert.equal((await fsp.stat(file)).size, item.size_bytes);
    assert.equal(await hash(file), item.sha256);
    return file;
  };
  const config = await json(await verified(path.join(directory, 'tts-runtime.json')));
  let profiles;
  const wheel = wheels?.find(
    (item) => path.basename(item.path) === 'puza_tts-0.3.0-py3-none-any.whl',
  );
  if (wheel) {
    const file = owned(path.join(storage, 'wheels'), path.resolve(root, wheel.path));
    assert.equal(await fsp.realpath(file), file);
    assert.equal((await fsp.stat(file)).size, wheel.size_bytes);
    assert.equal(await hash(file), wheel.sha256);
    const python = await verified(owned(directory, config.python_executable));
    const { stdout } = await promisify(execFile)(
      python,
      [
        '-I',
        '-S',
        '-B',
        '-c',
        wheelProfiles,
        file,
        JSON.stringify(engines.map((e) => e.profile_id)),
      ],
      { windowsHide: true, timeout: 30000, maxBuffer: 65536 },
    );
    profiles = JSON.parse(stdout);
  }
  const identities = [];
  for (const selection of engines) {
    let profile = profiles?.[selection.profile_id];
    if (!profile) {
      const file = await verified(
        owned(directory, path.join(config.profile_root, `${selection.profile_id}.json`)),
      );
      profile = { sha256: await hash(file), settings: await json(file) };
    }
    assert.equal(profile.settings.engine, selection.engine_id);
    const voiceFile = await verified(
      owned(directory, path.join(config.voice_store, selection.voice_id, 'voice.json')),
    );
    const voice = await json(voiceFile);
    const ref = voice.references.find(
      (r) => r.reference_id === selection.reference_id && r.engine_id === selection.engine_id,
    );
    assert.ok(ref, 'Pinned reference is missing');
    const audio = await verified(
      owned(path.dirname(voiceFile), path.join(path.dirname(voiceFile), ref.audio.path)),
    );
    assert.equal(await hash(audio), ref.audio.sha256);
    identities.push({
      ...selection,
      profile_sha256: profile.sha256,
      model_revision: profile.settings.model_revision,
      voice_manifest_sha256: await hash(voiceFile),
      reference_audio_sha256: ref.audio.sha256,
    });
  }
  return identities;
}
