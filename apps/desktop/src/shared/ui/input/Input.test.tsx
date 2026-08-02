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
    expect(input.getAttribute('aria-describedby')).toBe('email-error');
    expect(input.getAttribute('aria-errormessage')).toBe('email-error');
    expect(error.id).toBe('email-error');
    expect(error.textContent).toBe('Email is required');
  });

  it('links helper and error text separately when both are provided', () => {
    render(
      <Input
        id="name"
        label="Project name"
        helperText="Shown in the project list"
        errorText="Name is required"
        error
      />,
    );

    const input = screen.getByLabelText('Project name');

    expect(input.getAttribute('aria-describedby')).toBe('name-helper name-error');
    expect(screen.getByText('Shown in the project list').id).toBe('name-helper');
    expect(screen.getByRole('alert').id).toBe('name-error');
  });
});
