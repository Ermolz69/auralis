// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { ProjectPage } from './ProjectPage';

let pipelineStep: 'source' | 'subtitles' = 'subtitles';

vi.mock('@/shared/router', () => ({
  useNavigation: () => ({ pipelineStep }),
}));

vi.mock('../../../widgets/project-header', () => ({
  ProjectHeader: () => <header>Project header</header>,
}));

vi.mock('./SubtitleWorkspace', () => ({
  SubtitleWorkspace: () => <section data-testid="subtitle-workspace">Subtitle workspace</section>,
}));

vi.mock('./SourceWorkspace', () => ({
  SourceWorkspace: () => <section data-testid="source-workspace">Source workspace</section>,
}));

afterEach(() => {
  pipelineStep = 'subtitles';
  cleanup();
});

describe('ProjectPage', () => {
  it('lazy-loads the dedicated subtitle workspace for pipeline step two', async () => {
    render(<ProjectPage />);

    expect(screen.getByTestId('project-workspace').className).toContain('overflow-hidden');
    expect(await screen.findByTestId('subtitle-workspace')).not.toBeNull();
  });

  it('lazy-loads the source workspace for pipeline step one', async () => {
    pipelineStep = 'source';

    render(<ProjectPage />);

    expect(await screen.findByTestId('source-workspace')).not.toBeNull();
  });
});
