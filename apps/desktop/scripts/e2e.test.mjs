import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { chromium } from 'playwright';
import { installE2EFixture } from './e2e-fixture.mjs';
import { serveProduction } from './production-server.mjs';

const timestamp = '2026-09-03T12:00:00Z';
const youtubeProject = createProject({
  id: 'youtube-project',
  title: 'YouTube Project',
  status: 'ready_for_processing',
  source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=source' },
  metadata: createMetadata(),
});
const draftProject = createProject({ id: 'draft-project', title: 'Draft Project' });
const runningJob = createJob({
  id: 'running-job',
  projectId: youtubeProject.id,
  title: 'Running subtitle import',
  status: 'running',
  stage: 'extractOrGenerateTranscript',
  percent: 42,
  message: 'Downloading subtitles',
});
const completedJob = createJob({
  id: 'completed-job',
  projectId: youtubeProject.id,
  title: 'Completed media probe',
  status: 'completed',
  stage: 'fetchMetadata',
  percent: 100,
  message: 'Metadata ready',
});

let server;
let browser;

before(async () => {
  server = await serveProduction();
  browser = await chromium.launch({ headless: true });
});

after(async () => {
  await browser?.close();
  await server?.close();
});

e2e('01 loads the project list and global jobs through the production IPC boundary', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('button', { name: /^Open YouTube Project/ }).waitFor();
    await page.getByRole('button', { name: /^Open Draft Project/ }).waitFor();

    assert.equal(
      await page.getByRole('heading', { name: 'Projects', exact: true }).isVisible(),
      true,
    );
    assert.equal((await callsFor(page, 'list_projects_cmd')).length >= 1, true);
    assert.equal((await callsFor(page, 'list_jobs_cmd')).length >= 1, true);
  });
});

e2e('02 validates an empty project name without leaving the project list', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('button', { name: 'Create project' }).click();

    const input = page.getByRole('textbox', { name: 'Project name' });
    await page.getByText('Укажите название проекта', { exact: true }).waitFor();
    assert.equal(await input.getAttribute('aria-invalid'), 'true');
    assert.equal(
      await page.getByRole('heading', { name: 'Projects', exact: true }).isVisible(),
      true,
    );
    assert.equal((await callsFor(page, 'create_project_cmd')).length, 0);
  });
});

e2e('03 creates a project and opens its source workspace', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('textbox', { name: 'Project name' }).fill('Fresh Project');
    await page.getByRole('button', { name: 'Create project' }).click();

    await page.getByTestId('source-workspace').waitFor();
    await page.getByRole('heading', { name: 'Источник видео' }).waitFor();
    assert.deepEqual(await lastCallArgs(page, 'create_project_cmd'), { title: 'Fresh Project' });
    assert.equal(await page.getByText('Project created', { exact: true }).isVisible(), true);
  });
});

e2e('04 opens project metadata and returns to the project list', async () => {
  await scenario(baseSeed(), async (page) => {
    await openProject(page, 'YouTube Project');

    const metadata = page.getByLabel('Source metadata');
    await metadata.waitFor();
    assert.equal((await metadata.textContent()).includes('3:05'), true);
    assert.equal((await metadata.textContent()).includes('1920×1080'), true);
    assert.equal(
      await page.getByRole('textbox', { name: 'Video URL' }).inputValue(),
      youtubeProject.source.url,
    );

    await page.getByRole('button', { name: 'Back to projects' }).click();
    await page.getByRole('heading', { name: 'Projects', exact: true }).waitFor();
  });
});

e2e('05 persists the theme and installs a signed application update from settings', async () => {
  await scenario(
    baseSeed({
      updater: {
        currentVersion: '0.1.0',
        available: {
          version: '0.2.0',
          date: '2026-09-05T12:00:00Z',
          body: 'A verified release from GitHub Releases.',
        },
      },
    }),
    async (page) => {
      await page.getByText('Auralis 0.2.0 is available', { exact: true }).waitFor();
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
      await page.getByLabel('Color theme').selectOption('frost');

      await page.waitForFunction(() => document.documentElement.dataset.colorTheme === 'frost');
      assert.equal(
        await page.evaluate(() => localStorage.getItem('auralis:color-theme:v1')),
        'frost',
      );
      await page.getByRole('button', { name: 'Download and install 0.2.0' }).click();
      await page.getByText('Restarting', { exact: true }).waitFor();
      assert.equal(await page.evaluate(() => window.__e2e.restarted), true);
      assert.equal((await callsFor(page, 'plugin:updater|download_and_install')).length, 1);
      assert.equal((await callsFor(page, 'plugin:process|restart')).length, 1);
      await page.getByRole('button', { name: 'Back', exact: true }).click();
      await page.getByRole('heading', { name: 'Projects', exact: true }).waitFor();
    },
  );
});

e2e('06 returns from settings to the previously opened project', async () => {
  await scenario(baseSeed(), async (page) => {
    await openProject(page, 'YouTube Project');
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
    await page.getByRole('button', { name: 'Back', exact: true }).click();

    await page.getByTestId('source-workspace').waitFor();
    assert.equal(await page.getByRole('heading', { name: 'Источник видео' }).isVisible(), true);
  });
});

e2e('07 shows active and completed operations and restores focus after Escape', async () => {
  await scenario(baseSeed(), async (page) => {
    const queueButton = page.getByRole('button', { name: /Очередь/ });
    await queueButton.click();

    await page.getByRole('heading', { name: 'Job Queue' }).waitFor();
    assert.equal(
      await page
        .getByLabel('Active operations')
        .getByText(runningJob.title, { exact: true })
        .isVisible(),
      true,
    );
    assert.equal(
      await page
        .getByLabel('Operation history')
        .getByText(completedJob.title, { exact: true })
        .isVisible(),
      true,
    );
    await page.keyboard.press('Escape');
    await page.locator('#global-job-queue').waitFor({ state: 'detached' });
    assert.equal(await queueButton.evaluate((element) => element === document.activeElement), true);
  });
});

e2e('08 requests cancellation of an active job', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('button', { name: /Очередь/ }).click();
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();

    await page.getByText('Cancellation requested.', { exact: true }).waitFor();
    assert.deepEqual(await lastCallArgs(page, 'cancel_job_cmd'), { jobId: runningJob.id });
  });
});

e2e('09 navigates to subtitles and loads available YouTube tracks', async () => {
  await scenario(baseSeed(), async (page) => {
    await openSubtitleWorkspace(page);

    await page.getByRole('radiogroup').waitFor();
    assert.equal(await page.getByText('Русский (ru)', { exact: true }).isVisible(), true);
    assert.equal(await page.getByText('Английский (en)', { exact: true }).isVisible(), true);
    assert.deepEqual(await lastCallArgs(page, 'list_youtube_subtitle_tracks_cmd'), {
      projectId: youtubeProject.id,
    });
  });
});

e2e('10 filters subtitle tracks and selects a different language', async () => {
  await scenario(baseSeed(), async (page) => {
    await openSubtitleWorkspace(page);
    await page.getByRole('searchbox', { name: 'Поиск по доступным дорожкам' }).fill('English');

    await page.getByText('Английский (en)', { exact: true }).click();
    assert.equal(await page.locator('input[value="track-en"]').isChecked(), true);
    assert.equal(await page.getByText('Русский (ru)', { exact: true }).count(), 0);
  });
});

e2e('11 starts subtitle import with the selected track contract', async () => {
  await scenario(baseSeed({ jobs: [] }), async (page) => {
    await openSubtitleWorkspace(page);
    await page.getByRole('button', { name: 'Получить субтитры', exact: true }).click();
    await waitForCall(page, 'start_project_mock_pipeline_cmd');

    assert.deepEqual(await lastCallArgs(page, 'start_project_mock_pipeline_cmd'), {
      projectId: youtubeProject.id,
      subtitleTrackId: 'track-ru',
      subtitleLanguage: 'ru',
      subtitleAutoGenerated: false,
    });
    await page.getByText('Выполняется', { exact: true }).waitFor();
  });
});

e2e('12 imports a selected local video into a draft project', async () => {
  await scenario(baseSeed({ selectedFile: 'C:\\Videos\\local-clip.mp4' }), async (page) => {
    await openProject(page, 'Draft Project');
    await page.getByRole('button', { name: 'Import local video', exact: true }).click();

    await page.getByLabel('Connected video source').waitFor();
    assert.equal(
      await page
        .getByLabel('Connected video source')
        .getByText('local-clip.mp4', { exact: true })
        .isVisible(),
      true,
    );
    assert.deepEqual(await lastCallArgs(page, 'import_local_media_cmd'), {
      projectId: draftProject.id,
      path: 'C:\\Videos\\local-clip.mp4',
    });
    assert.equal((await callsFor(page, 'plugin:dialog|open')).length, 1);
  });
});

e2e('13 attaches a YouTube source to an existing draft', async () => {
  await scenario(baseSeed(), async (page) => {
    await openProject(page, 'Draft Project');
    const url = 'https://youtube.com/watch?v=new-source';
    await page.getByRole('textbox', { name: 'YouTube URL' }).fill(url);
    await page.getByRole('button', { name: 'Add from YouTube' }).click();

    await page.getByLabel('Connected video source').waitFor();
    assert.equal(await page.getByRole('textbox', { name: 'Video URL' }).inputValue(), url);
    assert.deepEqual(await lastCallArgs(page, 'create_project_from_youtube_cmd'), {
      url,
      projectId: draftProject.id,
    });
  });
});

e2e('14 resumes an unfinished YouTube import and publishes the project', async () => {
  const pending = { projectId: 'pending-project', title: 'Recovered Video', state: 'Failed' };
  await scenario(baseSeed({ pendingImports: [pending] }), async (page) => {
    await page.getByRole('region', { name: 'Pending YouTube imports' }).waitFor();
    await page.getByRole('button', { name: 'Resume', exact: true }).click();

    await page.getByText('YouTube import completed', { exact: true }).waitFor();
    await page.getByRole('button', { name: /^Open Recovered Video/ }).waitFor();
    assert.deepEqual(await lastCallArgs(page, 'resume_youtube_import_cmd'), {
      projectId: pending.projectId,
    });
  });
});

e2e('15 discards an unfinished YouTube import', async () => {
  const pending = { projectId: 'pending-project', title: 'Discard Me', state: 'Staged' };
  await scenario(baseSeed({ pendingImports: [pending] }), async (page) => {
    const pendingSection = page.getByRole('region', { name: 'Pending YouTube imports' });
    await pendingSection.waitFor();
    await page.getByRole('button', { name: 'Discard', exact: true }).click();

    await page.getByText('Pending import discarded', { exact: true }).waitFor();
    await pendingSection.waitFor({ state: 'detached' });
    assert.deepEqual(await lastCallArgs(page, 'discard_youtube_import_cmd'), {
      projectId: pending.projectId,
    });
  });
});

e2e('16 renames a project from its context menu', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('button', { name: /^Open YouTube Project/ }).click({ button: 'right' });
    await page.getByRole('menu', { name: 'Actions for YouTube Project' }).waitFor();
    page.once('dialog', (dialog) => dialog.accept('Renamed Project'));
    await page.getByRole('menuitem', { name: 'Переименовать' }).click();

    await page.getByRole('button', { name: /^Open Renamed Project/ }).waitFor();
    assert.deepEqual(await lastCallArgs(page, 'rename_project_cmd'), {
      projectId: youtubeProject.id,
      title: 'Renamed Project',
    });
  });
});

e2e('17 pins a project and exposes it in the sidebar', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('button', { name: /^Open YouTube Project/ }).click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Закрепить' }).click();

    await page.getByRole('button', { name: 'YouTube Project', exact: true }).waitFor();
    const preferences = await page.evaluate(() =>
      JSON.parse(localStorage.getItem('auralis.project-preferences.v1') ?? '{}'),
    );
    assert.equal(preferences[youtubeProject.id].pinned, true);
  });
});

e2e('18 cancels project deletion and keeps the project available', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('button', { name: 'Delete YouTube Project' }).click();
    const dialog = page.getByRole('dialog', { name: 'Delete Project' });
    await dialog.waitFor();
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();

    await dialog.waitFor({ state: 'hidden' });
    assert.equal(
      await page.getByRole('button', { name: /^Open YouTube Project/ }).isVisible(),
      true,
    );
    assert.equal((await callsFor(page, 'delete_project_cmd')).length, 0);
  });
});

e2e('19 confirms project deletion and removes it from recent projects', async () => {
  await scenario(baseSeed(), async (page) => {
    await page.getByRole('button', { name: 'Delete Draft Project' }).click();
    await page.getByRole('button', { name: 'Confirm Delete' }).click();

    await page.getByRole('button', { name: /^Open Draft Project/ }).waitFor({ state: 'detached' });
    assert.deepEqual(await lastCallArgs(page, 'delete_project_cmd'), {
      projectId: draftProject.id,
    });
    assert.equal(
      await page.getByRole('button', { name: /^Open YouTube Project/ }).isVisible(),
      true,
    );
  });
});

e2e('20 recovers the recent project list after a backend error', async () => {
  await scenario(
    baseSeed({
      failures: {
        list_projects_cmd: {
          times: 100,
          error: { code: 'IO', message: 'Project storage is temporarily unavailable' },
        },
      },
    }),
    async (page) => {
      await page.getByText('Could not load recent projects', { exact: true }).waitFor();
      assert.equal(
        await page
          .getByText('Project storage is temporarily unavailable', { exact: true })
          .isVisible(),
        true,
      );
      await page.evaluate(() => {
        window.__e2e.failures.list_projects_cmd.remaining = 0;
      });
      await page.getByRole('button', { name: 'Retry', exact: true }).click();

      await page.getByRole('button', { name: /^Open YouTube Project/ }).waitFor();
      assert.equal((await callsFor(page, 'list_projects_cmd')).length >= 2, true);
    },
  );
});

function e2e(name, run) {
  test(name, { timeout: 30_000 }, run);
}

async function scenario(seed, run) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();
  page.setDefaultTimeout(7_500);
  const pageErrors = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  await page.addInitScript(installE2EFixture, seed);

  try {
    await page.goto(server.url);
    await page.getByRole('heading', { name: 'Projects', exact: true }).waitFor();
    await run(page);
    assert.deepEqual(pageErrors, []);
    assert.deepEqual(await page.evaluate(() => window.__e2e.unknownCommands), []);
  } finally {
    await context.close();
  }
}

async function openProject(page, title) {
  await page.getByRole('button', { name: new RegExp(`^Open ${escapeRegExp(title)}`) }).click();
  await page.getByTestId('source-workspace').waitFor();
}

async function openSubtitleWorkspace(page) {
  await openProject(page, 'YouTube Project');
  await page.getByRole('button', { name: /Субтитры/ }).click();
  await page.getByTestId('subtitle-workspace').waitFor();
}

async function waitForCall(page, command) {
  await page.waitForFunction(
    (expectedCommand) => window.__e2e.calls.some((call) => call.command === expectedCommand),
    command,
  );
}

async function callsFor(page, command) {
  return page.evaluate(
    (expectedCommand) => window.__e2e.calls.filter((call) => call.command === expectedCommand),
    command,
  );
}

async function lastCallArgs(page, command) {
  const calls = await callsFor(page, command);
  assert.ok(calls.length > 0, `Expected ${command} to be called`);
  return calls.at(-1).args;
}

function baseSeed(overrides = {}) {
  return {
    projects: [youtubeProject, draftProject],
    jobs: [runningJob, completedJob],
    pendingImports: [],
    tracks: {
      [youtubeProject.id]: [
        {
          id: 'track-ru',
          language: 'ru',
          label: 'Russian',
          format: 'vtt',
          isAutoGenerated: false,
        },
        {
          id: 'track-en',
          language: 'en',
          label: 'English',
          format: 'vtt',
          isAutoGenerated: true,
        },
      ],
    },
    transcripts: {},
    selectedFile: null,
    failures: {},
    ...overrides,
  };
}

function createProject(overrides) {
  return {
    id: 'project',
    title: 'Project',
    status: 'draft',
    source: null,
    metadata: null,
    createdAt: timestamp,
    updatedAt: timestamp,
    ...overrides,
  };
}

function createMetadata() {
  return {
    durationMs: 185_000,
    width: 1920,
    height: 1080,
    fps: 30,
    videoCodec: 'h264',
    audioCodec: 'aac',
    sampleRate: 48_000,
    audioChannels: 2,
    container: 'mp4',
    hasVideo: true,
    hasAudio: true,
    audioTracks: [
      {
        streamIndex: 1,
        codec: 'aac',
        channels: 2,
        sampleRate: 48_000,
        language: 'en',
        isDefault: true,
      },
    ],
    streams: [
      { index: 0, codecType: 'video', codecName: 'h264', durationMs: 185_000 },
      { index: 1, codecType: 'audio', codecName: 'aac', durationMs: 185_000 },
    ],
  };
}

function createJob({ id, projectId, title, status, stage, percent, message }) {
  return {
    id,
    kind: 'dubbing',
    revision: 1,
    projectId,
    title,
    status,
    stage,
    progress: {
      percent,
      message,
      currentStep: stage,
      processedItems: null,
      totalItems: null,
    },
    error: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
