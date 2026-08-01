// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { Input } from './Input';

afterEach(() => cleanup());

describe('Input', () => {
  it('links helper errors to the control', () => {
    render(<Input id="email" label="Email" helperText="Email is required" error />);

    const input = screen.getByLabelText('Email');
    const error = screen.getByRole('alert');

    expect(input.getAttribute('aria-invalid')).toBe('true');
    expect(input.getAttribute('aria-describedby')).toBe('email-description');
    expect(error.id).toBe('email-description');
    expect(error.textContent).toBe('Email is required');
  });
});
