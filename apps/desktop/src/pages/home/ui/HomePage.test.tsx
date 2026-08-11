// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { HomePage } from './HomePage';

vi.mock('../../../features/project-list', () => ({
  ProjectList: () => <section aria-label="Recent Projects" />,
}));

describe('HomePage', () => {
  it('shows projects without a creation button', () => {
    render(<HomePage />);
    expect(screen.getByRole('heading', { name: 'Projects' })).not.toBeNull();
    expect(screen.getByLabelText('Recent Projects')).not.toBeNull();
    expect(screen.queryByRole('button', { name: 'New project' })).toBeNull();
  });
});
