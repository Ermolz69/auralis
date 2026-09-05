// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { FieldMessages } from './FieldMessages';
import { getFieldMessageState } from './fieldMessageState';

afterEach(() => cleanup());

describe('shared field messages', () => {
  it('uses helper text as an error when the field is invalid', () => {
    const state = getFieldMessageState('field', {
      helperText: 'Required',
      error: true,
    });

    render(
      <FieldMessages
        helperId={state.helperId}
        errorId={state.errorId}
        helperText={state.resolvedHelperText}
        errorText={state.resolvedErrorText}
      />,
    );

    expect(state.describedBy).toBe('field-error');
    expect(screen.getByRole('alert').textContent).toBe('Required');
  });

  it('keeps helper and explicit error messages linked in reading order', () => {
    const state = getFieldMessageState('email', {
      helperText: 'Use a work address',
      errorText: 'Address is invalid',
      error: true,
    });

    render(
      <FieldMessages
        helperId={state.helperId}
        errorId={state.errorId}
        helperText={state.resolvedHelperText}
        errorText={state.resolvedErrorText}
      />,
    );

    expect(state.describedBy).toBe('email-helper email-error');
    expect(screen.getByText('Use a work address').id).toBe('email-helper');
    expect(screen.getByRole('alert').id).toBe('email-error');
  });

  it('exposes helper text without error semantics for a valid field', () => {
    const state = getFieldMessageState('title', {
      helperText: 'Shown in the project list',
      error: false,
    });

    render(
      <FieldMessages
        helperId={state.helperId}
        errorId={state.errorId}
        helperText={state.resolvedHelperText}
        errorText={state.resolvedErrorText}
      />,
    );

    expect(state.describedBy).toBe('title-helper');
    expect(screen.getByText('Shown in the project list').id).toBe('title-helper');
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('returns no accessibility references when there are no messages', () => {
    const state = getFieldMessageState('empty', { error: false });
    const { container } = render(
      <FieldMessages
        helperId={state.helperId}
        errorId={state.errorId}
        helperText={state.resolvedHelperText}
        errorText={state.resolvedErrorText}
      />,
    );

    expect(state.describedBy).toBeUndefined();
    expect(state.helperId).toBeUndefined();
    expect(state.errorId).toBeUndefined();
    expect(container.childElementCount).toBe(0);
  });
});
