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

  it('renders categorized option groups and disabled empty states', () => {
    render(
      <Select
        label="Theme"
        defaultValue="frost"
        optionGroups={[
          {
            label: 'Light themes',
            options: [{ value: 'frost', label: 'Auralis Frost' }],
          },
          {
            label: 'Custom themes',
            options: [{ value: 'empty', label: 'No custom themes yet', disabled: true }],
          },
        ]}
      />,
    );

    expect(screen.getByRole('group', { name: 'Light themes' })).not.toBeNull();
    expect(screen.getByRole('group', { name: 'Custom themes' })).not.toBeNull();
    expect(
      (screen.getByRole('option', { name: 'No custom themes yet' }) as HTMLOptionElement).disabled,
    ).toBe(true);
  });
});
