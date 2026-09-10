import { spawn } from 'node:child_process';
import { rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const desktopRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const staticOutput = join(desktopRoot, 'storybook-visual-static');
const storybookCli = join(desktopRoot, 'node_modules', 'storybook', 'dist', 'bin', 'dispatcher.js');
const playwrightCli = join(desktopRoot, 'node_modules', 'playwright', 'cli.js');
const forwardedArguments = process.argv.slice(2).filter((argument) => argument !== '--');

let activeChild = null;

const runNode = (entrypoint, arguments_) =>
  new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [entrypoint, ...arguments_], {
      cwd: desktopRoot,
      env: { ...process.env, STORYBOOK_DISABLE_TELEMETRY: '1' },
      stdio: 'inherit',
    });
    activeChild = child;
    child.once('error', reject);
    child.once('exit', (code, signal) => {
      activeChild = null;
      if (signal) reject(new Error(`Process terminated by ${signal}`));
      else resolve(code ?? 1);
    });
  });

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.once(signal, () => {
    activeChild?.kill(signal);
    rmSync(staticOutput, { recursive: true, force: true });
    process.exit(signal === 'SIGINT' ? 130 : 143);
  });
}

let exitCode = 1;

try {
  rmSync(staticOutput, { recursive: true, force: true });
  const buildCode = await runNode(storybookCli, ['build', '--quiet', '--output-dir', staticOutput]);

  if (buildCode !== 0) {
    throw new Error(`Storybook static build failed with exit code ${buildCode}`);
  }

  exitCode = await runNode(playwrightCli, [
    'test',
    '--config',
    'playwright.storybook.config.mjs',
    ...forwardedArguments,
  ]);
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
} finally {
  rmSync(staticOutput, { recursive: true, force: true });
}

process.exitCode = exitCode;
