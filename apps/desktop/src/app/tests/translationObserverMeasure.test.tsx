/// <reference types="node" />
// @vitest-environment jsdom
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { act, cleanup, render } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TranslationRunControl } from '@/features/translation-run-control';
import { TranslationPublicationNotice } from '@/features/translation-review';

const api = vi.hoisted(() => ({
  listProjectTranslations: vi.fn(),
  getTranslationRunStatus: vi.fn(),
  getProjectTranslationSnapshot: vi.fn(),
  listTranslationHistory: vi.fn(),
  getTranslationPublication: vi.fn(),
  subscribeTranslationChanges: vi.fn(),
  subscribeTranslationPublicationChanges: vi.fn(),
  startTranslationRun: vi.fn(),
  pauseTranslationRun: vi.fn(),
  publishTranslationResult: vi.fn(),
}));
vi.mock('@/entities/translation', () => api);

const projectId = '00000000-0000-4000-8000-000000000001';
const resultId = '00000000-0000-4000-8000-000000000099';
const artifactId = '00000000-0000-4000-8000-000000000098';
const id = (value: number) => `00000000-0000-4000-8000-${String(value).padStart(12, '0')}`;
const commandNames = [
  'list_project_translations_cmd',
  'get_translation_run_status_cmd',
  'get_project_translation_snapshot_cmd',
  'list_translation_history_cmd',
  'get_translation_publication_cmd',
] as const;
type CommandName = (typeof commandNames)[number];
type Scenario =
  | 'empty'
  | 'one-active'
  | 'four-active'
  | 'empty-added'
  | 'one-progress'
  | 'four-progress'
  | 'event-burst'
  | 'lost-event'
  | 'publication';
type Sample = {
  scenario: Scenario;
  repetition: number;
  calls: Record<CommandName, number>;
  latencyMs: number | null;
};

let calls: Record<CommandName, number>;
let links: Array<Record<string, unknown>>;
let savedBlocks: number;
let publicationState: 'pending' | 'ready';
let runListeners: Set<() => void>;
let publicationListeners: Set<() => void>;

function link(index: number) {
  return {
    translationId: id(index + 2),
    projectId,
    sourceArtifactId: id(index + 20),
    sourceSha256: 'a'.repeat(64),
    sourceFormat: 'srt',
    sourceLanguage: 'zh',
    targetLanguage: 'ru',
    activeRunId: id(index + 10),
    selectedResultId: null,
    outputArtifactId: null,
    revision: 1,
    createdAt: '2026-10-16T00:00:00Z',
    updatedAt: '2026-10-16T00:00:00Z',
  };
}

function run(index: number) {
  return {
    runId: id(index + 10),
    phase: 'running',
    pauseRequested: false,
    savedBlocks,
    totalBlocks: 2,
    resultId: null,
    needsReview: null,
    activeHostJobId: id(index + 30),
  };
}

function response<T>(command: CommandName, value: T): Promise<T> {
  calls[command]++;
  const captured = structuredClone(value);
  return new Promise((done) => setTimeout(() => done(captured), 20));
}

function resetFixture(scenario: Scenario) {
  calls = Object.fromEntries(commandNames.map((name) => [name, 0])) as Record<CommandName, number>;
  links = Array.from(
    {
      length:
        scenario === 'four-active' || scenario === 'four-progress' || scenario === 'event-burst'
          ? 4
          : scenario === 'empty' || scenario === 'empty-added'
            ? 0
            : 1,
    },
    (_, index) => link(index),
  );
  savedBlocks = 0;
  publicationState = 'pending';
  runListeners = new Set();
  publicationListeners = new Set();
  api.listProjectTranslations.mockImplementation(() =>
    response('list_project_translations_cmd', links),
  );
  api.getTranslationRunStatus.mockImplementation((_projectId: string, translationId: string) => {
    const index = links.findIndex((item) => item.translationId === translationId);
    return response('get_translation_run_status_cmd', run(index));
  });
  api.getProjectTranslationSnapshot.mockImplementation(() =>
    response('get_project_translation_snapshot_cmd', {
      projectId,
      entries: links.map((item, index) => ({ link: item, run: run(index) })),
    }),
  );
  api.listTranslationHistory.mockImplementation(() =>
    response('list_translation_history_cmd', { entries: [], nextCursor: null }),
  );
  api.getTranslationPublication.mockImplementation(() =>
    response('get_translation_publication_cmd', {
      link: {
        ...links[0],
        selectedResultId: publicationState === 'ready' ? resultId : null,
        outputArtifactId: publicationState === 'ready' ? artifactId : null,
      },
      publication: { resultId, state: publicationState },
    }),
  );
  api.subscribeTranslationChanges.mockImplementation(
    async (_projectId: string, notify: () => void) => {
      runListeners.add(notify);
      return () => runListeners.delete(notify);
    },
  );
  api.subscribeTranslationPublicationChanges.mockImplementation(
    async (_projectId: string, notify: () => void) => {
      publicationListeners.add(notify);
      return () => publicationListeners.delete(notify);
    },
  );
}

async function advance(milliseconds: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(milliseconds);
  });
}

async function untilVisible(text: string): Promise<number> {
  const started = Date.now();
  for (let elapsed = 0; elapsed <= 5000; elapsed += 10) {
    if (document.body.textContent?.includes(text)) return Date.now() - started;
    await advance(10);
  }
  throw new Error(`Committed value did not appear: ${text}`);
}

async function measure(scenario: Scenario, repetition: number): Promise<Sample> {
  vi.useFakeTimers();
  vi.setSystemTime(0);
  resetFixture(scenario);
  const view =
    scenario === 'publication'
      ? render(
          <TranslationPublicationNotice
            projectId={projectId}
            translationId={String(links[0].translationId)}
            resultId={resultId}
            refreshRevision={0}
          />,
        )
      : render(<TranslationRunControl projectId={projectId} />);
  await advance(100);
  await advance(900);
  let latencyMs: number | null = null;
  if (scenario === 'empty-added') {
    links = [link(0)];
    for (const listener of runListeners) listener();
    latencyMs = await untilVisible('0/2');
  } else if (
    scenario === 'one-progress' ||
    scenario === 'four-progress' ||
    scenario === 'event-burst' ||
    scenario === 'lost-event'
  ) {
    savedBlocks = 1;
    if (scenario === 'event-burst')
      for (let index = 0; index < 20; index++) for (const listener of runListeners) listener();
    else if (scenario !== 'lost-event') for (const listener of runListeners) listener();
    latencyMs = await untilVisible('1/2');
  } else if (scenario === 'publication') {
    publicationState = 'ready';
    for (const listener of publicationListeners) listener();
    latencyMs = await untilVisible('выбрана для проекта');
  }
  await advance(Math.max(0, 10_000 - Date.now()));
  const sample = { scenario, repetition, calls: { ...calls }, latencyMs };
  view.unmount();
  cleanup();
  vi.useRealTimers();
  return sample;
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('translation observer measurement', () => {
  it('records the frozen project scenarios', async () => {
    const samples: Sample[] = [];
    for (const scenario of [
      'empty',
      'one-active',
      'four-active',
      'empty-added',
      'one-progress',
      'four-progress',
      'event-burst',
      'lost-event',
      'publication',
    ] as const) {
      for (let repetition = 1; repetition <= 3; repetition++)
        samples.push(await measure(scenario, repetition));
    }
    expect(samples).toHaveLength(27);
    expect(
      samples.every((sample) => Object.values(sample.calls).every((count) => count >= 0)),
    ).toBe(true);
    const report = process.env.AURALIS_TRANSLATION_OBSERVER_REPORT;
    if (report) {
      const destination = resolve('../..', report);
      mkdirSync(dirname(destination), { recursive: true });
      writeFileSync(
        destination,
        `${JSON.stringify({ protocol: 'translation-observer-v2', ipcDelayMs: 20, windowMs: 10_000, samples }, null, 2)}\n`,
      );
    }
  });
});
