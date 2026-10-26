import { spawn } from 'node:child_process';
import { inventory, stopOwned } from './processes.mjs';
import { OwnedTree } from './lineage.mjs';
import { UnconfirmedCleanup } from './core.mjs';

export function installerBudget(remaining, signal) {
  const timeout = Math.min(300000, remaining());
  signal?.throwIfAborted();
  if (!Number.isFinite(timeout) || timeout <= 0)
    throw new Error('Offline installer deadline exhausted before spawn');
  return timeout;
}

export async function runInstaller(command, args, options, remaining, signal, onOwned) {
  installerBudget(remaining, signal);
  const baseline = await inventory();
  // Budget and abort are rechecked after the inventory and before creating a child.
  const timeout = installerBudget(remaining, signal);
  const child = spawn(command, args, options);
  const tree = child.pid ? new OwnedTree(child.pid, baseline) : null;
  let timer, observer, observation, ownershipError;
  const onAbort = () => {
    child.kill();
  };
  signal?.addEventListener('abort', onAbort, { once: true });
  if (signal?.aborted) onAbort();
  const completion = new Promise((resolve, reject) => {
    timer = setTimeout(() => {
      child.kill();
      reject(new Error('Offline component installer deadline exhausted'));
    }, timeout);
    child.once('error', reject);
    child.once('exit', (code) => {
      if (signal?.aborted) reject(signal.reason);
      else if (code === 0) resolve();
      else
        reject(new Error('Offline component installation failed; inspect component-install.log'));
    });
  });
  // Register rejection handling while the first process inventory is pending.
  completion.catch(() => {});
  try {
    if (!tree) return await completion;
    tree.observe(await inventory());
    tree.assertKnown();
    const processes = async () => {
      const active = tree.observe(await inventory());
      tree.assertKnown();
      return active;
    };
    await onOwned?.(processes);
    observer = setInterval(() => {
      if (!observation)
        observation = processes()
          .catch((error) => {
            ownershipError = error;
            child.kill();
          })
          .finally(() => {
            observation = null;
          });
    }, 1000);
    await completion;
    if (ownershipError) throw new UnconfirmedCleanup(ownershipError.message);
  } finally {
    clearTimeout(timer);
    clearInterval(observer);
    signal?.removeEventListener('abort', onAbort);
    await observation;
    try {
      if (child.pid) {
        if (child.exitCode === null && child.signalCode === null) child.kill();
        const result = await stopOwned(child, tree);
        if (!result.confirmed)
          throw new UnconfirmedCleanup('Offline installer process release is unconfirmed');
      }
    } catch (error) {
      throw new UnconfirmedCleanup(`Offline installer cleanup: ${error.message}`);
    } finally {
      onOwned?.(null);
    }
  }
}
