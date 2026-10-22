import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import type { SpeechSourceSnapshot } from '@/shared/api/contracts/speech';
import { SpeechSourcePreview } from './SpeechSourcePreview';

afterEach(cleanup);

const source: SpeechSourceSnapshot = {
  kind: 'translation',
  source_id: 'source-1',
  revision: 'revision-1',
  sha256: 'a'.repeat(64),
  text: 'A model-free 600-cue engineering fixture.',
  cues: Array.from({ length: 600 }, (_, index) => ({
    id: `cue-${index + 1}`,
    role: `role-${index + 1}`,
    text: `Реплика ${index + 1}`,
    start_ms: index * 2000,
    end_ms: index * 2000 + 1500,
  })),
};
const props = {
  source,
  voices: [
    {
      voice_id: 'fixture',
      display_name: 'Test voice',
      description: '',
      manifest_sha256: 'b'.repeat(64),
      references: [
        {
          reference_id: 'neutral',
          engine_id: 'voxcpm2',
          emotion_id: 'neutral',
          transcript: 'fixture',
          audio_sha256: 'c'.repeat(64),
          qualification_status: 'qualified' as const,
        },
      ],
    },
  ],
  engineId: 'voxcpm2',
  assignments: [],
  assign: vi.fn(),
};

it('bounds both role and cue controls and preserves absolute ordering across all 600 cues', () => {
  render(<SpeechSourcePreview {...props} />);
  expect(screen.getAllByLabelText(/^Голос реплики /)).toHaveLength(50);
  expect(screen.getAllByLabelText(/^Голос роли /)).toHaveLength(50);
  expect(screen.queryByText('Реплика 51')).toBeNull();
  const visited: string[] = [];
  const next = screen.getByRole('button', { name: 'Следующая страница: Реплики' });
  for (let page = 0; page < 12; page++) {
    const texts = screen.getAllByText(/^Реплика \d+$/).map((item) => item.textContent!);
    expect(texts).toHaveLength(50);
    expect(texts[0]).toBe(`Реплика ${page * 50 + 1}`);
    visited.push(...texts);
    if (page < 11) fireEvent.click(next);
  }
  expect(new Set(visited).size).toBe(600);
  expect(visited).toEqual(source.cues.map((cue) => cue.text));
  expect(next).toHaveProperty('disabled', true);
  expect(screen.getByText('600 · 1198.00–1199.50 с')).not.toBeNull();
});

it('keeps source IDs and controlled assignments across page changes and parent refreshes', () => {
  const assign = vi.fn();
  const { rerender } = render(<SpeechSourcePreview {...props} assign={assign} />);
  fireEvent.click(screen.getByRole('button', { name: 'Следующая страница: Реплики' }));
  fireEvent.change(screen.getByLabelText('Голос реплики cue-51'), { target: { value: 'fixture' } });
  expect(assign).toHaveBeenCalledWith('cue', 'cue-51', 'fixture');
  rerender(
    <SpeechSourcePreview
      {...props}
      assign={assign}
      assignments={[
        { scope: 'cue', target: 'cue-51', voice_id: 'fixture', reference_id: 'neutral' },
      ]}
    />,
  );
  expect(screen.getByLabelText('Голос реплики cue-51')).toHaveProperty('value', 'fixture');
  fireEvent.click(screen.getByRole('button', { name: 'Предыдущая страница: Реплики' }));
  fireEvent.click(screen.getByRole('button', { name: 'Следующая страница: Реплики' }));
  expect(screen.getByLabelText('Голос реплики cue-51')).toHaveProperty('value', 'fixture');
});

it('resets pages for a new source revision and supports bounded page sizes without dropping the first cue', () => {
  const { rerender } = render(<SpeechSourcePreview {...props} />);
  fireEvent.click(screen.getByRole('button', { name: 'Следующая страница: Роли' }));
  fireEvent.click(screen.getByRole('button', { name: 'Следующая страница: Реплики' }));
  fireEvent.change(screen.getByLabelText('Реплики на странице'), { target: { value: '100' } });
  expect(screen.getAllByLabelText(/^Голос реплики /)).toHaveLength(100);
  expect(screen.getByText('Реплика 1')).not.toBeNull();
  rerender(
    <SpeechSourcePreview
      {...props}
      source={{ ...source, revision: 'revision-2', cues: source.cues.slice(0, 2) }}
    />,
  );
  expect(screen.getByLabelText('Голос роли role-1')).not.toBeNull();
  expect(screen.getByLabelText('Голос реплики cue-1')).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Следующая страница: Реплики' })).toHaveProperty(
    'disabled',
    true,
  );
  expect(screen.getByLabelText('Реплики на странице')).toHaveProperty('value', '50');
});
