// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { ExportPanel } from './ExportPanel';

afterEach(() => cleanup());

describe('ExportPanel', () => {
  it('does not present video export as an available action', () => {
    render(<ExportPanel />);

    const exportButton = screen.getByRole<HTMLButtonElement>('button', {
      name: 'Export unavailable',
    });

    expect(exportButton.disabled).toBe(true);
    expect(screen.getByText('Video export is not available in the current app contract.')).not.toBeNull();
    expect(exportButton.getAttribute('aria-describedby')).toBe('export-unavailable-note');
  });
});
