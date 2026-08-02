// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { Button } from './Button';

afterEach(() => cleanup());

describe('Button', () => {
  it('marks loading actions as busy and disabled', () => {
    render(<Button loading>Save</Button>);

    const button = screen.getByRole<HTMLButtonElement>('button', { name: /save/i });

    expect(button.disabled).toBe(true);
    expect(button.getAttribute('aria-busy')).toBe('true');
    expect(button.textContent).toBe('Save');
    expect(screen.queryByText('Loading')).toBeNull();
  });
});
