import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const STARTED = 'translation_model_hash_started';
const FINISHED = 'translation_model_hash_finished';

export function parseHashRecords(text) {
  const lines = text.split('\n');
  lines.pop();
  return lines
    .filter((line) => line.trim())
    .map((line) => JSON.parse(line))
    .filter((record) => [STARTED, FINISHED].includes(record.fields?.event_name))
    .map((record) => ({ ...record.fields, timestamp: record.timestamp ?? null }));
}

export function readHashRecords(dataRoot) {
  const directory = path.join(dataRoot, 'logs');
  if (!fs.existsSync(directory)) return [];
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .filter(
      (entry) =>
        entry.isFile() && (entry.name === 'auralis.log' || entry.name.startsWith('auralis.log.')),
    )
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) =>
      parseHashRecords(fs.readFileSync(path.join(directory, entry.name), 'utf8')),
    );
}

function verifyCounters(record) {
  for (const field of ['hash_operation', 'hashed_bytes', 'total_bytes']) {
    assert(Number.isSafeInteger(record[field]), `Missing or invalid hash ${field}`);
  }
  assert(record.hash_operation > 0);
  assert(record.total_bytes > 0);
  assert(record.hashed_bytes > 0 && record.hashed_bytes <= record.total_bytes);
}

export function observedOpenHash(records, expectedBytes) {
  assert(Number.isSafeInteger(expectedBytes) && expectedBytes > 0);
  const candidates = records.filter(
    (record) => record.event_name === STARTED && record.total_bytes === expectedBytes,
  );
  const open = candidates.filter(
    (record) =>
      !records.some(
        (item) => item.event_name === FINISHED && item.hash_operation === record.hash_operation,
      ),
  );
  assert(open.length <= 1, 'Ambiguous unfinished model hashes');
  if (!open.length) return null;
  const started = open[0];
  verifyCounters(started);
  assert(
    started.hashed_bytes < expectedBytes,
    'Hash was already fully read before pause observation',
  );
  return started;
}

export function interruptedHash(records, started) {
  verifyCounters(started);
  assert.equal(started.event_name, STARTED);
  const matching = records.filter(
    (record) => record.event_name === FINISHED && record.hash_operation === started.hash_operation,
  );
  assert(matching.length <= 1, 'Duplicate model hash terminal records');
  if (!matching.length) return null;
  const finished = matching[0];
  verifyCounters(finished);
  assert.equal(finished.total_bytes, started.total_bytes);
  assert.equal(finished.outcome, 'failed');
  assert(
    finished.hashed_bytes >= started.hashed_bytes && finished.hashed_bytes < finished.total_bytes,
  );
  return finished;
}

export function verifyFreshHashes(records, interrupted) {
  const starts = records.filter(
    (record) =>
      record.event_name === STARTED &&
      record.total_bytes === interrupted.started.total_bytes &&
      record.hash_operation > interrupted.started.hash_operation,
  );
  assert(starts.length > 0, 'Resume reused an incomplete model hash');
  const operations = new Set();
  return starts.map((started) => {
    verifyCounters(started);
    assert(!operations.has(started.hash_operation), 'Duplicate fresh model hash start');
    operations.add(started.hash_operation);
    const finished = records.filter(
      (record) =>
        record.event_name === FINISHED && record.hash_operation === started.hash_operation,
    );
    assert.equal(finished.length, 1, 'Fresh model hash is unfinished or duplicated');
    verifyCounters(finished[0]);
    assert.equal(finished[0].outcome, 'completed');
    assert.equal(finished[0].total_bytes, started.total_bytes);
    assert.equal(finished[0].hashed_bytes, started.total_bytes);
    return { started, finished: finished[0] };
  });
}
