import fsp from 'node:fs/promises';
import path from 'node:path';
import { uniqueRun, storage, json, hash, validateSpec, Blocked, idPattern } from './core.mjs';
import { setTimeout as delay } from 'node:timers/promises';
import { Report } from './report.mjs';
import { candidateIdentity, DesktopSession, lockSession } from './session.mjs';
import { requireQuiescent } from './processes.mjs';
import { smoke } from './smoke.mjs';
import { story, storySteps, videoSteps } from './story.mjs';
import { seedProfile } from './seed.mjs';
import { verifyAssembly, verifySources, playback } from './media.mjs';
import { acquireDownloadAdmission } from './admission.mjs';

const mode = process.argv[2];
if (!['smoke', 'real'].includes(mode)) throw new Error('Expected smoke or real');
const run = await uniqueRun(`desktop-${mode}`);
const report = new Report(run, `desktop-${mode}`, [
  'preflight',
  'seed',
  'launch',
  ...(mode === 'smoke'
    ? ['empty-projects', 'input-validation', 'source-rejection', 'settings-and-return']
    : storySteps),
  'javascript-errors',
  'cleanup',
]);
let release, session, spec, candidate;
const downloadLeases = new Set();
const admitDownload = async () => {
  const finish = await acquireDownloadAdmission();
  downloadLeases.add(finish);
  return async () => {
    await requireQuiescent();
    const result = await finish();
    downloadLeases.delete(finish);
    return result;
  };
};
let completed = false;
let deadline = Date.now() + (mode === 'smoke' ? 300000 : 28800000);
const remaining = () => {
  const ms = deadline - Date.now();
  if (ms <= 0) throw new Error('Frozen case deadline exhausted');
  return ms;
};
try {
  if (
    await report.step('preflight', async () => {
      if (process.platform !== 'win32' || process.arch !== 'x64')
        throw new Blocked('Windows x64 is required');
      release = await lockSession();
      if (mode === 'real') {
        spec = validateSpec(await json(process.argv[3], 32 * 1024 * 1024));
        if (spec.export_video === true)
          for (const id of videoSteps) {
            const index = report.data.steps.findIndex(
              (step) => step.id === id.replace(/-video$/, '-assembly'),
            );
            report.data.steps.splice(index + 1, 0, { id, status: 'not_run' });
          }
        deadline = Date.now() + spec.max_wall_seconds * 1000;
        if (typeof spec.edit_text !== 'string' || !spec.edit_text.trim())
          throw new Error('Pin the edit text');
        const used = path.join(storage, 'cases');
        await fsp.mkdir(used, { recursive: true });
        await fsp.writeFile(
          path.join(used, `${spec.case_id}.json`),
          JSON.stringify({ run_id: run.id, spec_sha256: await hash(process.argv[3]) }),
          { flag: 'wx' },
        );
        await fsp.copyFile(process.argv[3], path.join(run.directory, 'spec.json'));
      }
      if (mode === 'smoke' && process.argv[3]) {
        spec = await json(process.argv[3], 32 * 1024 * 1024);
        if (
          spec.mode !== 'component-smoke' ||
          spec.allow_inference !== false ||
          !idPattern.test(spec.case_id ?? '') ||
          spec.max_attempts !== 1 ||
          !spec.runtime_seed ||
          !spec.component_wheels ||
          spec.max_wall_seconds !== 1800
        )
          throw new Error('Expected a frozen model-free component smoke case');
        deadline = Date.now() + spec.max_wall_seconds * 1000;
        const used = path.join(storage, 'cases');
        await fsp.mkdir(used, { recursive: true });
        await fsp.writeFile(
          path.join(used, `${spec.case_id}.json`),
          JSON.stringify({ run_id: run.id, spec_sha256: await hash(process.argv[3]) }),
          { flag: 'wx' },
        );
        await fsp.copyFile(process.argv[3], path.join(run.directory, 'spec.json'));
        report.data.steps.splice(
          report.data.steps.findIndex((step) => step.id === 'javascript-errors'),
          0,
          { id: 'components-connected', status: 'not_run' },
        );
      }
      candidate = await candidateIdentity();
      if (spec && candidate.receipt.sha256 !== spec.candidate_sha256)
        throw new Error('The frozen case belongs to another candidate');
      report.data.versions = candidate.receipt;
      return requireQuiescent();
    })
  ) {
    if (await report.step('seed', () => (spec ? seedProfile(spec, run) : { seeded: false }))) {
      session = new DesktopSession(run, candidate);
      if (await report.step('launch', () => session.start(remaining))) {
        if (mode === 'smoke') {
          await smoke(session.page, report);
          if (spec?.mode === 'component-smoke')
            await report.step(
              'components-connected',
              async () => {
                const stop = Date.now() + 5000;
                while (!session.log.includes('contract_snapshot_verified') && Date.now() < stop)
                  await delay(100);
                if (!session.log.includes('contract_snapshot_verified'))
                  throw new Error(
                    'No verified CONTRACT boundary after ordinary UI source submission',
                  );
                return {
                  contract_snapshot_verified: true,
                  model_inference: false,
                  profiles_copied: true,
                  voices_modified: false,
                };
              },
              session.page,
            );
        } else {
          const budgetEnd = Date.now() + spec.max_wall_seconds * 1000;
          const budget = () => {
            const ms = Math.min(remaining(), budgetEnd - Date.now());
            if (ms <= 0) throw new Error('Frozen case deadline exhausted');
            return ms;
          };
          completed = await story(
            session.page,
            report,
            spec,
            budget,
            requireQuiescent,
            admitDownload,
            () => verifySources(path.join(run.directory, 'profile'), spec),
          );
          if (completed) {
            await report.step(
              'verify-artifacts',
              () =>
                verifyAssembly(
                  path.join(run.directory, 'profile'),
                  spec.engines,
                  spec.export_video === true,
                ),
              session.page,
            );
            await report.step('restart-playback', async () => {
              const old = await session.close();
              if (!old.confirmed || old.forced)
                throw new Error('Ordinary close did not complete gracefully');
              await session.start(remaining);
              const page = session.page;
              await page
                .getByRole('button', { name: /^Open / })
                .filter({ hasText: /.*/ })
                .first()
                .click();
              await page.getByRole('button', { name: /Текст, субтитры и озвучка/ }).click();
              const raw = page.getByLabel('Готовый исходный синтез', { exact: true });
              const options = await raw
                .locator('option')
                .evaluateAll((nodes) => nodes.filter((n) => n.value).map((n) => n.value));
              if (!options.length) throw new Error('Ready speech disappeared after restart');
              await raw.selectOption(options[0]);
              const assembly = page.getByLabel('Сохранённые сборки', { exact: true });
              const saved = await assembly
                .locator('option')
                .evaluateAll((nodes) => nodes.filter((n) => n.value).map((n) => n.value));
              if (spec.export_video === true) {
                const video = await assembly
                  .locator('option')
                  .evaluateAll(
                    (nodes, rawId) =>
                      nodes.find(
                        (node) =>
                          node.value &&
                          node.textContent.includes('MP4') &&
                          node.textContent.includes(`источник ${rawId.slice(0, 8)}`),
                      )?.value,
                    options[0],
                  );
                if (!video) throw new Error('Ready video disappeared after restart');
                await assembly.selectOption(video);
              } else await assembly.selectOption(saved[0]);
              await page
                .getByRole('button', { name: 'Прослушать итоговый WAV', exact: true })
                .click();
              const wav = await playback(page, 'Итоговое аудио');
              if (spec.export_video === true) {
                await page
                  .getByRole('button', { name: 'Воспроизвести русское видео', exact: true })
                  .click();
                return { wav, video: await playback(page, 'Видео с русской озвучкой') };
              }
              return wav;
            });
          }
        }
        await report.step(
          'javascript-errors',
          () => {
            if (session.errors.length) throw new Error(session.errors.join('\n'));
            return { errors: 0 };
          },
          session.page,
        );
      }
    }
  }
} catch (error) {
  report.data.steps.push({
    id: 'runner-error',
    status: 'failed',
    error: String(error.message).slice(0, 8000),
  });
} finally {
  let confirmed = !session?.child;
  await report.step('cleanup', async () => {
    if (session) {
      const result = await session.close();
      for (const finish of downloadLeases) {
        await finish();
        downloadLeases.delete(finish);
      }
      confirmed = result.confirmed && downloadLeases.size === 0;
      if (result.forced)
        throw new Error('Forced application shutdown; inspect retained diagnostics');
      return result;
    }
    return { confirmed: true, launched: false };
  });
  if (release) await release(confirmed && !report.data.cleanup_unconfirmed);
  await report.save();
  process.stdout.write(`UI report: ${run.id}\n`);
  process.exitCode = report.exitCode();
}
