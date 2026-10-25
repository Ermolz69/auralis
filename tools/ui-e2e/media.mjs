import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { hash, owned, json } from './core.mjs';
import { inspectWav } from './wav.mjs';
import { createHash } from 'node:crypto';
import { verifySpeechEvidence } from './speech-evidence.mjs';

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
      const audio = [...document.querySelectorAll('audio,video')].find(
        (element) => element.getAttribute('aria-label') === label,
      );
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
    video_width: element.videoWidth ?? null,
    video_height: element.videoHeight ?? null,
  }));
}
export async function verifyAssembly(profile, selections, expectVideo = false) {
  const db = new DatabaseSync(path.join(profile, 'auralis.sqlite'), { readOnly: true });
  let publications;
  try {
    publications = db
      .prepare(
        'SELECT p.publication_json,i.plan_json,i.plan_sha256,i.source_run_id FROM audio_assembly_publications p JOIN audio_assembly_intents i USING(assembly_id) WHERE p.ready=1',
      )
      .all();
    assert.ok(publications.length > 0, 'No ready assembly publication');
    const facts = [];
    const videoEngines = new Set();
    for (const row of publications) {
      const publication = JSON.parse(row.publication_json);
      const plan = JSON.parse(row.plan_json);
      const raw = db
        .prepare('SELECT plan_json,plan_sha256 FROM speech_run_intents WHERE run_id=?')
        .get(row.source_run_id);
      const speech = JSON.parse(raw.plan_json);
      const digest = (value) => createHash('sha256').update(value).digest('hex');
      assert.equal(digest(raw.plan_json), raw.plan_sha256);
      assert.equal(digest(row.plan_json), row.plan_sha256);
      const reviewed = db
        .prepare(
          'SELECT r.snapshot_json FROM speech_text_reviews r JOIN speech_run_reviews s USING(decision_id) WHERE s.run_id=?',
        )
        .get(row.source_run_id);
      assert.ok(reviewed, 'Speech has no saved approval snapshot');
      const review = JSON.parse(reviewed.snapshot_json);
      verifySpeechEvidence(speech, review, selections);
      if (review.source.kind === 'translation') {
        const decision = db
          .prepare(
            'SELECT decision FROM translation_review_events WHERE project_id=? AND translation_id=? AND result_sha256=? ORDER BY sequence DESC LIMIT 1',
          )
          .get(review.project_id, review.source.source_id, review.source.sha256);
        assert.equal(decision?.decision, 'approved', 'The translated source is not approved');
      }
      if (plan.video) {
        assert.equal(publication.files.length, 4);
        assert.equal(publication.files.filter((file) => file.kind === 'video').length, 1);
        assert.equal(publication.files.filter((file) => file.kind === 'video_manifest').length, 1);
        videoEngines.add(speech.profile.engine_id);
      }
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
      assert.equal(publication.full_scale_samples, 0);
      assert.ok(publication.peak_sample_abs < 32768);
      for (const file of [
        ...publication.files,
        plan.raw_wav,
        plan.raw_manifest,
        plan.layout,
        ...(plan.video ? [plan.video.artifact] : []),
      ]) {
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
        if (file.kind === 'video_manifest') {
          const metadata = await json(target, 16 * 1024 * 1024);
          assert.equal(metadata.policy, plan.video.policy_id);
          assert.equal(metadata.assembly_plan_sha256, row.plan_sha256);
          assert.equal(metadata.source.sha256, plan.video.artifact.sha256);
          assert.equal(
            metadata.output_sha256,
            publication.files.find((item) => item.kind === 'video').sha256,
          );
          assert.equal(metadata.speech_frames, publication.frames);
          assert.equal(metadata.decoded_full_scale_samples, 0);
          assert.ok(Math.abs(metadata.decoded_frames - plan.video.media.duration_ms * 48) <= 2048);
          assert.equal(metadata.media.video_codec, 'h264');
          assert.equal(metadata.media.audio_codec, 'aac');
          assert.ok(Math.abs(metadata.media.duration_ms - plan.video.media.duration_ms) <= 200);
        }
        facts.push({
          artifact_id: file.artifact_id,
          kind: file.kind,
          sha256: file.sha256,
          size_bytes: file.size_bytes,
        });
      }
    }
    if (expectVideo)
      for (const selection of selections)
        assert.ok(
          videoEngines.has(selection.engine_id),
          `Missing ready video for ${selection.engine_id}`,
        );
    return {
      ready_assemblies: publications.length,
      video_engines: [...videoEngines],
      files: facts,
    };
  } finally {
    db.close();
  }
}
