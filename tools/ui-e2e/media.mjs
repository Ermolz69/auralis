import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { hash, owned } from './core.mjs';
import { inspectWav } from './wav.mjs';

export async function verifySources(profile, spec) {
  const db = new DatabaseSync(path.join(profile, 'auralis.sqlite'), { readOnly: true });
  let artifacts;
  try {
    artifacts = db.prepare("SELECT * FROM artifacts WHERE state='ready'").all();
  } finally {
    db.close();
  }
  const selected = artifacts.filter((item) =>
    /^(originalsubtitle|downloadedvideo)$/i.test(item.kind.replace(/_/g, '')),
  );
  assert.equal(selected.length, 2, 'Expected separate managed video and Chinese subtitle sources');
  const facts = [];
  const managed = path.join(profile, 'projects');
  for (const artifact of selected) {
    assert.equal(artifact.location_kind, 'StorageKey');
    const file = owned(managed, path.join(managed, artifact.location_value));
    assert.equal(await fsp.realpath(file), file);
    const bytes = (await fsp.stat(file)).size;
    assert.ok(bytes > 0 && bytes <= 1073741824, 'Acquired source exceeds its frozen byte budget');
    const sha256 = await hash(file);
    if (/subtitle/i.test(artifact.kind))
      assert.equal(sha256, spec.source_subtitle_sha256, 'Chinese subtitle source changed');
    facts.push({ artifact_id: artifact.id, kind: artifact.kind, sha256, size_bytes: bytes });
  }
  return { files: facts };
}

export async function playback(page, label) {
  const audio = page.getByLabel(label, { exact: true });
  await audio.waitFor();
  await audio.evaluate((element) => {
    element.muted = true;
  });
  const initial = await audio.evaluate((element) => element.currentTime);
  await page.waitForFunction(
    ({ label, initial }) => {
      const audio = document.querySelector(`audio[aria-label="${label}"]`);
      return audio && audio.readyState >= 2 && !audio.error && audio.currentTime > initial + 0.15;
    },
    { label, initial },
    { timeout: 15000 },
  );
  return audio.evaluate((element) => ({
    duration_seconds: element.duration,
    current_time: element.currentTime,
    ready_state: element.readyState,
    error: element.error?.code ?? null,
  }));
}
export async function verifyAssembly(profile, selections) {
  const db = new DatabaseSync(path.join(profile, 'auralis.sqlite'), { readOnly: true });
  let publications;
  try {
    publications = db
      .prepare(
        'SELECT p.publication_json,i.plan_json,i.source_run_id FROM audio_assembly_publications p JOIN audio_assembly_intents i USING(assembly_id) WHERE p.ready=1',
      )
      .all();
    assert.ok(publications.length > 0, 'No ready assembly publication');
    const facts = [];
    for (const row of publications) {
      const publication = JSON.parse(row.publication_json);
      const plan = JSON.parse(row.plan_json);
      const raw = db
        .prepare('SELECT plan_json,plan_sha256 FROM speech_run_intents WHERE run_id=?')
        .get(row.source_run_id);
      const speech = JSON.parse(raw.plan_json);
      const ids = speech.utterances.map((item) => item.utterance_id);
      assert.deepEqual(plan.timeline.expected_utterance_ids, ids);
      assert.deepEqual(
        plan.timeline.utterances.map((item) => item.utterance_id),
        ids,
      );
      assert.equal(new Set(ids).size, ids.length);
      assert.equal(plan.timeline.source_plan_sha256, raw.plan_sha256);
      assert.equal(
        db.prepare('SELECT ready FROM speech_publications WHERE run_id=?').get(row.source_run_id)
          ?.ready,
        1,
      );
      assert.ok(speech.review.decision_id);
      for (const utterance of speech.utterances)
        assert.ok(
          selections.some(
            (item) =>
              item.engine_id === utterance.engine_id &&
              item.profile_id === utterance.profile_id &&
              item.voice_id === utterance.voice_id &&
              item.reference_id === utterance.reference_id,
          ),
          'Speech selection differs from the frozen case',
        );
      assert.equal(publication.full_scale_samples, 0);
      assert.ok(publication.peak_sample_abs < 32768);
      for (const file of [...publication.files, plan.raw_wav, plan.raw_manifest, plan.layout]) {
        const artifact = db
          .prepare('SELECT * FROM artifacts WHERE id=? AND state=?')
          .get(file.artifact_id, 'ready');
        assert.equal(artifact?.location_kind, 'StorageKey');
        assert.equal(artifact.project_id, plan.project_id);
        const managed = path.join(profile, 'projects');
        const target = owned(managed, path.join(managed, artifact.location_value));
        assert.equal(await fsp.realpath(target), target);
        assert.equal((await fsp.stat(target)).size, file.size_bytes);
        assert.equal(await hash(target), file.sha256);
        if (file.kind === 'wav') {
          const measured = await inspectWav(target);
          for (const [key, value] of Object.entries(measured))
            assert.equal(value, publication[key], `Published WAV ${key} differs`);
        }
        facts.push({
          artifact_id: file.artifact_id,
          kind: file.kind,
          sha256: file.sha256,
          size_bytes: file.size_bytes,
        });
      }
    }
    return { ready_assemblies: publications.length, files: facts };
  } finally {
    db.close();
  }
}
