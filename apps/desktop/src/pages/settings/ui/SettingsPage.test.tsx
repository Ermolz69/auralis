// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { SettingsPage } from './SettingsPage';

afterEach(() => cleanup());

describe('SettingsPage', () => {
  it('marks settings sections as unavailable instead of interactive preferences', () => {
    render(<SettingsPage />);

    expect(screen.getByLabelText('Appearance settings unavailable')).not.toBeNull();
    expect(screen.getByLabelText('Export defaults unavailable')).not.toBeNull();
    expect(screen.getAllByText('Unavailable')).toHaveLength(2);
    expect(screen.getAllByText(/not part of the current app contract/i)).toHaveLength(2);
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });
});
