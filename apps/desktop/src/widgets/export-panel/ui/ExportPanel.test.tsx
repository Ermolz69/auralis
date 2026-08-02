// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { ExportPanel } from './ExportPanel';

afterEach(() => cleanup());

describe('ExportPanel', () => {
  it('does not present video export as an available action', () => {
    render(<ExportPanel />);

    expect(screen.queryByRole('button', { name: 'Export unavailable' })).toBeNull();

    const status = screen.getByRole('status');
    expect(status.textContent).toContain('Export unavailable');
    expect(screen.getByText('Export is not available in this version.')).not.toBeNull();
    expect(status.getAttribute('aria-describedby')).toBe('export-unavailable-note');
  });
});
