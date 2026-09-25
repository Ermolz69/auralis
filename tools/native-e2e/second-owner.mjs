import fs from 'node:fs/promises';
import path from 'node:path';

const STARTUP_TIMEOUT_MS = 60_000;
const REJECTION_TIMEOUT_MS = 30_000;
const POLL_INTERVAL_MS = 100;
const LEASE_CONFLICT = 'This Auralis storage is already open in another instance';

export async function verifySecondOwnerRejected(dataRoot, firstApp, launchSecond) {
  const checkpointPath = path.join(dataRoot, 'native-e2e-bootstrap.txt');
  await waitForCheckpoint(checkpointPath, firstApp.child, 'setup-complete', STARTUP_TIMEOUT_MS);
  let second;
  try {
    second = launchSecond();
    await waitForExit(second.child, REJECTION_TIMEOUT_MS);
    assert(second.child.exitCode !== 0, 'Second native desktop exited successfully with an occupied data root');
    assert(second.child.signalCode === null, 'Second native desktop needed forced termination');
    const diagnostics = `${second.stderr}\n${second.stdout}`;
    assert(diagnostics.includes(LEASE_CONFLICT), `Second native desktop did not report the storage lease conflict: ${diagnostics}`);
    const checkpoints = await fs.readFile(checkpointPath, 'utf8');
    assert(count(checkpoints, 'setup-started') === 2, 'Second desktop never reached storage lease acquisition');
    assert(count(checkpoints, 'paths-ready') === 1, 'Second desktop acquired the occupied storage lease');
    assert(count(checkpoints, 'setup-complete') === 1, 'Second desktop completed setup against the occupied data root');
    assert(isRunning(firstApp.child), 'First native desktop stopped during the second-owner rejection');
  } finally {
    if (second && isRunning(second.child)) {
      second.child.kill('SIGKILL');
      await waitForExit(second.child, REJECTION_TIMEOUT_MS);
    }
  }
}

async function waitForCheckpoint(checkpointPath, child, expected, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    assert(isRunning(child), 'First native desktop exited before setup completed');
    const checkpoints = await readCheckpoint(checkpointPath);
    if (checkpoints.includes(expected)) return;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error(`First native desktop did not reach ${expected}`);
}

async function readCheckpoint(checkpointPath) {
  try {
    return await fs.readFile(checkpointPath, 'utf8');
  } catch (error) {
    if (error?.code === 'ENOENT') return '';
    throw error;
  }
}

async function waitForExit(child, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!isRunning(child)) return;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Second native desktop did not exit after its storage lease conflict');
}

function count(checkpoints, value) {
  return checkpoints.split(/\r?\n/).filter((line) => line === value).length;
}

function isRunning(child) {
  return child.exitCode === null && child.signalCode === null;
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
