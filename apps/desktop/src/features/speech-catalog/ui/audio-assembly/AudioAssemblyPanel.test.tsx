import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { AudioAssemblyPanel } from './AudioAssemblyPanel';
import type {
  AudioAssemblyPreview as AssemblyPreview,
  AudioAssemblyView as AssemblyView,
} from '@/shared/api/contracts/audioAssembly';
import type { SpeechRunView } from '@/shared/api/contracts/speech';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => `asset:${path}` }));
const hash = 'a'.repeat(64);
const source: SpeechRunView = {
  run_id: 'raw',
  profile_id: 'fixture',
  utterance_count: 1,
  chunk_count: 1,
  job: null,
  result: { run_id: 'raw', attempt_id: 'raw-attempt', duration_ms: 100, ready: true, files: [] },
};
const preview: AssemblyPreview = {
  source_run_id: 'raw',
  source_plan_sha256: hash,
  preview_sha256: hash,
  profile_id: 'fixture',
  settings: { mode: 'continuous', source_duration_ms: null, pause_after_ms: [0] },
  sample_rate: 48000,
  channels: 1,
  frames: 4800,
  utterances: [
    {
      utterance_id: 'one',
      audio_sha256: hash,
      sample_rate: 48000,
      channels: 1,
      frame_count: 4800,
      source_window: { start_ms: 100, end_ms: 500 },
      pause_after_ms: 0,
    },
  ],
  conflicts: [],
  renderer_available: true,
  renderer_error: null,
};
const planned: AssemblyView = {
  assembly_id: 'assembly',
  source_run_id: 'raw',
  plan_sha256: hash,
  settings: preview.settings,
  utterance_count: 1,
  frames: 4800,
  job: null,
  result: null,
};
const job = {
  id: 'assembly-job',
  kind: 'audioAssembly',
  revision: 2,
  projectId: 'project',
  title: 'Assembly',
  status: 'running' as const,
  stage: 'postprocessAudio' as const,
  progress: {
    percent: 35,
    message: 'Assembly fixture',
    currentStep: null,
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-10-23T00:00:00Z',
  updatedAt: '2026-10-23T00:00:01Z',
};
let saved: AssemblyView | null;
beforeEach(() => {
  vi.clearAllMocks();
  saved = null;
  invoke.mockImplementation(async (command: string) => {
    if (command === 'list_audio_assemblies_cmd') return saved ? [saved] : [];
    if (command === 'inspect_audio_assembly_cmd') return preview;
    if (command === 'prepare_audio_assembly_cmd') {
      saved = planned;
      return saved;
    }
    if (command === 'start_audio_assembly_cmd') {
      saved = { ...planned, job };
      return saved;
    }
    if (command === 'cancel_job_cmd') {
      saved = { ...planned, job: { ...job, status: 'cancelled' } };
      return saved.job;
    }
    if (command === 'get_audio_assembly_cmd' || command === 'verify_audio_assembly_cmd')
      return saved;
    if (command === 'resume_audio_assembly_cmd') {
      saved = {
        ...planned,
        job: { ...job, status: 'completed', progress: { ...job.progress, percent: 100 } },
        result: {
          assembly_id: 'assembly',
          attempt_id: 'attempt',
          sample_rate: 48000,
          channels: 1,
          frames: 4800,
          pcm_sha256: hash,
          peak_sample_abs: 100,
          full_scale_samples: 0,
          ready: true,
          files: [
            { artifact_id: 'assembled-wav', kind: 'wav', sha256: hash, size_bytes: 9644 },
            { artifact_id: 'assembly-manifest', kind: 'manifest', sha256: hash, size_bytes: 100 },
          ],
        },
      };
      return saved;
    }
    if (command === 'resolve_artifact_path_cmd') return 'managed/assembled.wav';
    throw new Error(`Unexpected fixture command ${command}`);
  });
});
afterEach(cleanup);

async function prepare() {
  fireEvent.click(screen.getByRole('button', { name: 'Измерить и показать тайминг' }));
  fireEvent.click(await screen.findByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить сборку' }));
  await screen.findByRole('button', { name: 'Собрать WAV' });
}

it('binds explicit review to exact timing and discards review after a pause edit', async () => {
  render(<AudioAssemblyPanel projectId="project" source={source} />);
  expect(screen.getByRole('button', { name: 'Подготовить сборку' })).toHaveProperty(
    'disabled',
    true,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Измерить и показать тайминг' }));
  fireEvent.click(await screen.findByRole('checkbox'));
  expect(screen.getByRole('button', { name: 'Подготовить сборку' })).toHaveProperty(
    'disabled',
    false,
  );
  fireEvent.change(screen.getByLabelText('Пауза после one, мс'), { target: { value: '25' } });
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(screen.getByRole('button', { name: 'Подготовить сборку' })).toHaveProperty(
    'disabled',
    true,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Измерить и показать тайминг' }));
  fireEvent.click(await screen.findByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить сборку' }));
  await screen.findByRole('button', { name: 'Собрать WAV' });
  expect(invoke).toHaveBeenCalledWith('prepare_audio_assembly_cmd', {
    projectId: 'project',
    request: {
      source: { source_run_id: 'raw', settings: { ...preview.settings, pause_after_ms: [25] } },
      expected_preview_sha256: hash,
      reviewed: true,
    },
  });
});

it('uses the common cancel, resumes a saved intent and verifies only the assembled playback', async () => {
  render(<AudioAssemblyPanel projectId="project" source={source} />);
  await prepare();
  fireEvent.click(screen.getByRole('button', { name: 'Собрать WAV' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Отменить сборку' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Продолжить сборку' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Прослушать итоговый WAV' }));
  expect(screen.getByLabelText('Сохранённые сборки')).toHaveProperty('disabled', true);
  await waitFor(() =>
    expect(screen.getByLabelText('Итоговое аудио').getAttribute('src')).toBe(
      'asset:managed/assembled.wav',
    ),
  );
  expect(invoke).toHaveBeenCalledWith('cancel_job_cmd', { jobId: 'assembly-job' });
  expect(invoke).toHaveBeenCalledWith('resume_audio_assembly_cmd', {
    projectId: 'project',
    assemblyId: 'assembly',
  });
  expect(invoke).toHaveBeenCalledWith('verify_audio_assembly_cmd', {
    projectId: 'project',
    assemblyId: 'assembly',
  });
  expect(invoke).toHaveBeenCalledWith('resolve_artifact_path_cmd', { artifactId: 'assembled-wav' });
});

it('shows an impossible fit and requires a declared standalone duration before inspecting windows', async () => {
  const previous = invoke.getMockImplementation()!;
  invoke.mockImplementation((command, args) =>
    command === 'inspect_audio_assembly_cmd'
      ? Promise.resolve({
          ...preview,
          frames: null,
          conflicts: [
            {
              kind: 'impossible_fit',
              utterance_id: 'one',
              required_frames: 4800,
              available_frames: 48,
            },
          ],
        })
      : previous(command, args),
  );
  render(<AudioAssemblyPanel projectId="project" source={source} />);
  fireEvent.change(screen.getByLabelText('Режим тайминга'), {
    target: { value: 'subtitle_window' },
  });
  expect(screen.getByRole('button', { name: 'Измерить и показать тайминг' })).toHaveProperty(
    'disabled',
    true,
  );
  fireEvent.change(screen.getByLabelText('Длительность отдельной аудиолинии, мс'), {
    target: { value: '500' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Измерить и показать тайминг' }));
  await screen.findByText(/one: речь 100.0 мс, доступно 1.0 мс/);
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(screen.getByRole('button', { name: 'Подготовить сборку' })).toHaveProperty(
    'disabled',
    true,
  );
});

it('keeps a missing managed tool actionable and cannot prepare a render', async () => {
  const previous = invoke.getMockImplementation()!;
  invoke.mockImplementation((command, args) =>
    command === 'inspect_audio_assembly_cmd'
      ? Promise.resolve({
          ...preview,
          renderer_available: false,
          renderer_error: 'Восстановите установку FFmpeg.',
        })
      : previous(command, args),
  );
  render(<AudioAssemblyPanel projectId="project" source={source} />);
  fireEvent.click(screen.getByRole('button', { name: 'Измерить и показать тайминг' }));
  await screen.findByText('Восстановите установку FFmpeg.');
  fireEvent.click(screen.getByRole('checkbox'));
  expect(screen.getByRole('button', { name: 'Подготовить сборку' })).toHaveProperty(
    'disabled',
    true,
  );
});
