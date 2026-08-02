// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { Textarea } from './Textarea';

afterEach(() => cleanup());

describe('Textarea', () => {
  it('links helper errors to the control', () => {
    render(<Textarea id="notes" label="Notes" helperText="Notes are too long" error />);

    const textarea = screen.getByLabelText('Notes');
    const error = screen.getByRole('alert');

    expect(textarea.getAttribute('aria-invalid')).toBe('true');
    expect(textarea.getAttribute('aria-describedby')).toBe('notes-error');
    expect(textarea.getAttribute('aria-errormessage')).toBe('notes-error');
    expect(error.id).toBe('notes-error');
  });
});
