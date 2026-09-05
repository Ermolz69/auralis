// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { StateView } from './StateView';

afterEach(() => cleanup());

describe('StateView', () => {
  it('shares loading semantics and optional content', () => {
    render(
      <StateView
        icon="Inbox"
        title="Loading"
        description="Please wait"
        action={<button type="button">Cancel</button>}
        loading
        role="status"
        live="polite"
      />,
    );

    expect(screen.getByRole('status').getAttribute('aria-busy')).toBe('true');
    expect(screen.getByText('Please wait')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeTruthy();
  });

  it('renders a compact danger state without loading-only semantics', () => {
    render(
      <StateView
        title="Could not load"
        description="Try again later"
        tone="danger"
        density="compact"
        role="alert"
      />,
    );

    const alert = screen.getByRole('alert');
    const title = screen.getByText('Could not load');
    const description = screen.getByText('Try again later');

    expect(alert.hasAttribute('aria-busy')).toBe(false);
    expect(title.className).toContain('text-sm');
    expect(title.className).not.toContain('animate-pulse');
    expect(description.className).toContain('text-danger');
    expect(description.className).toContain('text-xs');
  });

  it('omits optional description and action nodes', () => {
    const { container } = render(<StateView title="Nothing here" />);

    expect(container.textContent).toBe('Nothing here');
    expect(container.querySelector('button')).toBeNull();
  });
});
