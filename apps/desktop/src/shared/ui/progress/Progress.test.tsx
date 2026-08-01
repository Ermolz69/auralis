// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { Progress } from './Progress';

afterEach(() => cleanup());

describe('Progress', () => {
  it('has an accessible fallback name and percent text', () => {
    render(<Progress value={25} />);

    const progress = screen.getByRole('progressbar', { name: 'Progress' });

    expect(progress.getAttribute('aria-valuenow')).toBe('25');
    expect(progress.getAttribute('aria-valuetext')).toBe('25%');
  });

  it('announces indeterminate loading without a misleading value', () => {
    render(<Progress indeterminate aria-label="Import progress" />);

    const progress = screen.getByRole('progressbar', { name: 'Import progress' });

    expect(progress.getAttribute('aria-valuenow')).toBeNull();
    expect(progress.getAttribute('aria-valuetext')).toBe('Loading');
    expect(progress.getAttribute('aria-busy')).toBe('true');
  });
});
