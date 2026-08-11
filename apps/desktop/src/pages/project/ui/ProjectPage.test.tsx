// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { ProjectPage } from './ProjectPage';

vi.mock('@/shared/router', () => ({
  useNavigation: () => ({ pipelineStep: 'subtitles' }),
}));

vi.mock('../../../widgets/project-header', () => ({
  ProjectHeader: () => <header>Project header</header>,
}));

vi.mock('./SubtitleWorkspace', () => ({
  SubtitleWorkspace: () => <section data-testid="subtitle-workspace">Subtitle workspace</section>,
}));

afterEach(() => cleanup());

describe('ProjectPage', () => {
  it('renders the dedicated subtitle workspace for pipeline step two', () => {
    render(<ProjectPage />);

    expect(screen.getByTestId('project-workspace').className).toContain('overflow-hidden');
    expect(screen.getByTestId('subtitle-workspace')).not.toBeNull();
  });
});
