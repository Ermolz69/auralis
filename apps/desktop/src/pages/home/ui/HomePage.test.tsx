// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { HomePage } from './HomePage';
import { ProjectProvider } from '@/entities/project';
import { NavigationProvider } from '@/shared/router';

vi.mock('@/shared/api/tauri', () => ({
  invoke: vi.fn(),
  listen: vi.fn(async () => () => {}),
}));

vi.mock('../../../features/project-list', () => ({
  ProjectList: () => <section aria-label="Recent Projects" />,
}));

describe('HomePage', () => {
  it('shows projects and source-based creation controls', () => {
    render(
      <ProjectProvider>
        <NavigationProvider>
          <HomePage />
        </NavigationProvider>
      </ProjectProvider>,
    );
    expect(screen.getByRole('heading', { name: 'Projects' })).not.toBeNull();
    expect(screen.getByLabelText('Recent Projects')).not.toBeNull();
    expect(screen.getByRole('heading', { name: 'Add a source' })).not.toBeNull();
    expect(screen.getByLabelText('YouTube URL')).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Add from YouTube' })).toHaveProperty(
      'disabled',
      true,
    );
    expect(screen.queryByRole('button', { name: 'New project' })).toBeNull();
  });
});
