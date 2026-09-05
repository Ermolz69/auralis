// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { Notice } from './Notice';

afterEach(() => cleanup());

describe('Notice', () => {
  it('provides one semantic notice layout for alerts', () => {
    render(
      <Notice icon="CircleAlert" title="Outdated" tone="warning" role="alert" live="assertive">
        Refreshing data.
      </Notice>,
    );

    expect(screen.getByRole('alert').textContent).toContain('Outdated');
    expect(screen.getByRole('alert').textContent).toContain('Refreshing data.');
  });

  it('supports a titleless neutral notice without forced live-region semantics', () => {
    const { container } = render(<Notice icon="Info">Informational text.</Notice>);
    const notice = container.firstElementChild;

    expect(notice?.textContent).toContain('Informational text.');
    expect(notice?.className).toContain('border-muted/40');
    expect(notice?.hasAttribute('role')).toBe(false);
    expect(notice?.hasAttribute('aria-live')).toBe(false);
    expect(container.querySelector('p')).toBeNull();
  });

  it('applies accent styling and status announcements', () => {
    render(
      <Notice icon="CircleCheck" title="Saved" tone="accent" role="status" live="polite">
        Your changes are safe.
      </Notice>,
    );

    const status = screen.getByRole('status');
    expect(status.getAttribute('aria-live')).toBe('polite');
    expect(status.className).toContain('border-accent/40');
    expect(status.textContent).toContain('Saved');
  });
});
