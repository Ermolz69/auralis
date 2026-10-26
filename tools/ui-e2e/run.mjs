import fsp from 'node:fs/promises';
import path from 'node:path';
import {
  uniqueRun,
  storage,
  json,
  hash,
  validateSpec,
  Blocked,
  idPattern,
  root,
  owned,
} from './core.mjs';
import { setTimeout as delay } from 'node:timers/promises';
import { Report } from './report.mjs';
import { candidateIdentity, DesktopSession, lockSession } from './session.mjs';
import { requireQuiescent } from './processes.mjs';
import { smoke } from './smoke.mjs';
import { story, storySteps, workflowSteps } from './story.mjs';
import { seedProfile } from './seed.mjs';
import { verifyAssembly, verifySources } from './media.mjs';
import { acquireDownloadAdmission } from './admission.mjs';
import { reopenPlayback, workflowRecords } from './reopen.mjs';
import { ResourceObserver } from './resources.mjs';
import { setObservationSignal } from './observe.mjs';
import { selectionIdentities, verifySelectionPins } from './selections.mjs';
import { verifyPcmReadOnly } from './pcm.mjs';

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
  'resource-observations',
  'cleanup',
  'session-release',
]);
let release, session, spec, candidate, observer, watchdog, stopReason, installerProcesses;
const abort = new AbortController();
setObservationSignal(abort.signal);
const stop = async (error) => {
  stopReason ??= error;
  report.abortReason = stopReason;
  abort.abort(stopReason);
  if (session?.page && !session.page.isClosed()) {
    for (const name of ['Отменить', 'Отменить сборку', 'Пауза']) {
      const cancel = session.page.getByRole('button', { name, exact: true });
      if (await cancel.isVisible().catch(() => false))
        await cancel.click({ timeout: 5000 }).catch(() => {});
    }
    await session.page
      .screenshot({ path: path.join(run.directory, 'budget-stop.png'), timeout: 5000 })
      .catch(() => {});
    await session.page.close().catch(() => {});
  }
};
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
  if (stopReason) throw stopReason;
  observer?.check();
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
        report.data.steps.splice(
          3,
          storySteps.length,
          ...workflowSteps(spec).map((id) => ({ id, status: 'not_run' })),
        );
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
      candidate = await candidateIdentity(storage, remaining);
      if (spec?.allow_inference && candidate.receipt.version !== 3)
        throw new Error('Inference requires the complete version 3 build input manifest');
      if (spec?.allow_inference)
        for (const input of candidate.receipt.build_inputs.sources.filter((item) =>
          item.path.startsWith('tools/ui-e2e/'),
        ))
          if ((await hash(owned(root, path.join(root, input.path)), remaining)) !== input.sha256)
            throw new Error('UI runner differs from the frozen candidate build inputs');
      if (
        spec?.candidate_receipt_sha256 &&
        candidate.receipt_sha256 !== spec.candidate_receipt_sha256
      )
        throw new Error('Frozen candidate build receipt changed');
      if (spec && candidate.receipt.sha256 !== spec.candidate_sha256)
        throw new Error('The frozen case belongs to another candidate');
      report.data.versions = candidate.receipt;
      const idle = await requireQuiescent();
      if (spec?.allow_inference) {
        observer = new ResourceObserver(
          run,
          spec.resource_budget,
          async () => {
            const app = session?.tree ? await session.processes() : [];
            const extra = installerProcesses ? await installerProcesses() : [];
            return [
              ...new Map([...app, ...extra].map((p) => [`${p.pid}:${p.created}`, p])).values(),
            ];
          },
          stop,
        );
        await observer.sample(true);
        observer.start();
      }
      watchdog = setInterval(() => {
        if (Date.now() >= deadline && !stopReason)
          void stop(new Error('Frozen case hard deadline exhausted'));
      }, 1000);
      report.data.coverage = {
        case_scope: spec?.case_scope ?? mode,
        real_models: spec?.allow_inference === true,
        native_cancellation: 'not_run',
        native_corruption: 'not_run',
        native_restart_during_job: 'not_run',
        ...(spec?.case_scope === 'diagnostic'
          ? { translation: 'not_run', mp4: 'not_run', tts_utterances: 1 }
          : {}),
      };
      return idle;
    })
  ) {
    if (
      await report.step('seed', async () => {
        const seeded = spec
          ? await seedProfile(spec, run, remaining, abort.signal, (processes) => {
              installerProcesses = processes;
            })
          : { seeded: false };
        if (observer) {
          await observer.auditProfile();
          seeded.engine_identities = await selectionIdentities(
            path.join(run.directory, 'profile'),
            spec.engines,
            remaining,
          );
          verifySelectionPins(seeded.engine_identities, spec.engine_identities);
        }
        return seeded;
      })
    ) {
      session = new DesktopSession(run, candidate, {
        tts_execution_budget_seconds: spec?.tts_execution_budget_seconds,
        tts_runtime_sha256: report.data.steps.find((step) => step.id === 'seed').evidence
          ?.tts_runtime_sha256,
      });
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
            () =>
              verifySources(path.join(run.directory, 'profile'), spec, { candidate, remaining }),
          );
          if (completed) {
            if (
              await report.step(
                'verify-artifacts',
                () =>
                  verifyAssembly(
                    path.join(run.directory, 'profile'),
                    spec.engine_identities ?? spec.engines,
                    spec.export_video === true,
                    spec,
                    remaining,
                    {
                      ui_approval: report.data.steps.find((s) => s.id === 'translate-edit-approve')
                        ?.evidence,
                      source_artifact_id: report.data.steps
                        .find((s) => s.id === 'source-identities')
                        ?.evidence?.files?.find((f) => /subtitle/i.test(f.kind ?? ''))?.artifact_id,
                      pcm: (input) =>
                        verifyPcmReadOnly(
                          path.join(run.directory, 'profile'),
                          run,
                          input,
                          spec,
                          remaining,
                          abort.signal,
                          (processes) => {
                            installerProcesses = processes;
                          },
                          admitDownload,
                        ),
                    },
                  ),
                session.page,
              )
            ) {
              await report.step('restart-playback', async () => {
                const old = await session.close();
                if (!old.confirmed || old.forced)
                  throw new Error('Ordinary close did not complete gracefully');
                await session.start(remaining);
                return reopenPlayback(session.page, workflowRecords(report, spec), remaining);
              });
            }
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
  clearInterval(watchdog);
  let confirmed = !session?.child;
  await report.step('cleanup', async () => {
    if (session) {
      const result = await session.close();
      for (const finish of downloadLeases) {
        await requireQuiescent();
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
  await report.step('resource-observations', async () => {
    if (!observer) return { required: false, model_inference: false };
    await observer.stop();
    let facts;
    try {
      if (confirmed && !observer.failure) await observer.auditProfile();
    } finally {
      facts = await observer.stop();
      report.data.resources = facts;
    }
    observer.check();
    return facts;
  });
  await report.step('session-release', async () => {
    const safe = confirmed && !report.data.cleanup_unconfirmed;
    if (release) await release(safe);
    return { acquired: !!release, retained: !!release && !safe };
  });
  await report.save();
  process.stdout.write(`UI report: ${run.id}\n`);
  process.exitCode = report.exitCode();
}
