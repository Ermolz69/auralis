import assert from 'node:assert/strict';
import { test } from 'node:test';
import { runInstaller } from './installer.mjs';
import { inventory } from './processes.mjs';

test(
  'abort stops the observed owned installer process without launching a component',
  { skip: process.platform !== 'win32', timeout: 30000 },
  async () => {
    const abort = new AbortController();
    const reason = new Error('Model-free installer abort probe');
    let owned;
    await assert.rejects(
      runInstaller(
        process.execPath,
        ['-e', 'setTimeout(()=>{},60000)'],
        { windowsHide: true, stdio: 'ignore' },
        () => 15000,
        abort.signal,
        async (processes) => {
          if (processes) {
            const records = await processes();
            assert.ok(records.length >= 1);
            owned = records.find((record) => /node\.exe$/i.test(record.name));
            assert.ok(owned, 'The installer root identity was not observed');
            abort.abort(reason);
          }
        },
      ),
      (error) => error === reason,
    );
    assert.ok(owned?.pid);
    assert.equal(
      (await inventory()).some((p) => p.pid === owned.pid && p.created === owned.created),
      false,
    );
  },
);
