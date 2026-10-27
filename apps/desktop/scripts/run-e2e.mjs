import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const packageDir = fileURLToPath(new URL('../', import.meta.url));
const runId = `${new Date().toISOString().replace(/[:.]/g, '-')}-${randomUUID().slice(0, 8)}`;
const resultsDir = path.join(packageDir, 'e2e-results', runId);
const testFile = fileURLToPath(new URL('./e2e.test.mjs', import.meta.url));
const forwardedArgs = process.argv.slice(2);

mkdirSync(resultsDir, { recursive: true });
writeFileSync(
  path.join(resultsDir, 'run.json'),
  JSON.stringify({ run_id: runId, layer: 'fixture-ui-guided' }),
);

const result = spawnSync(
  process.execPath,
  [
    '--test',
    '--test-concurrency=1',
    '--test-reporter=spec',
    '--test-reporter-destination=stdout',
    '--test-reporter=junit',
    `--test-reporter-destination=${path.join(resultsDir, 'junit.xml')}`,
    ...forwardedArgs,
    testFile,
  ],
  {
    cwd: packageDir,
    env: { ...process.env, AURALIS_BROWSER_E2E_RESULTS: resultsDir },
    stdio: 'inherit',
  },
);

if (result.error) throw result.error;
if (result.signal) {
  process.stderr.write(`E2E runner stopped by ${result.signal}.\n`);
  process.exitCode = 1;
} else {
  process.exitCode = result.status ?? 1;
}
