import { spawnSync } from 'node:child_process';
import { mkdirSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const packageDir = fileURLToPath(new URL('../', import.meta.url));
const resultsDir = path.join(packageDir, 'e2e-results');
const testFile = fileURLToPath(new URL('./e2e.test.mjs', import.meta.url));
const forwardedArgs = process.argv.slice(2);

rmSync(resultsDir, { recursive: true, force: true });
mkdirSync(resultsDir, { recursive: true });

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
    env: process.env,
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
