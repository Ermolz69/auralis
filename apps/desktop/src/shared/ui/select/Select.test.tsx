// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { Select } from './Select';

afterEach(() => cleanup());

describe('Select', () => {
  it('links helper errors to the control', () => {
    render(
      <Select
        id="language"
        label="Language"
        helperText="Choose a language"
        error
        options={[{ value: 'en', label: 'English' }]}
      />,
    );

    const select = screen.getByLabelText('Language');
    const error = screen.getByRole('alert');

    expect(select.getAttribute('aria-invalid')).toBe('true');
    expect(select.getAttribute('aria-describedby')).toBe('language-error');
    expect(select.getAttribute('aria-errormessage')).toBe('language-error');
    expect(error.id).toBe('language-error');
  });
});
