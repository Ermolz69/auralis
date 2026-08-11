// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Toaster } from './Toaster';
import { toast, TOAST_EXIT_DURATION } from './toast';

describe('Toaster', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    toast.dismissAll();
    vi.runAllTimers();
  });

  afterEach(() => {
    cleanup();
    toast.dismissAll();
    vi.runAllTimers();
    vi.useRealTimers();
  });

  it('shows countdown progress and keeps the toast mounted during its exit animation', () => {
    const { container } = render(<Toaster />);

    act(() => {
      toast.warning('Укажите название проекта', { duration: 1000 });
    });

    const message = screen.getByText('Укажите название проекта');
    const item = message.closest('[data-toast-phase]');
    const countdown = container.querySelector('.toast-countdown');
    expect(item?.getAttribute('data-toast-phase')).toBe('visible');
    expect((countdown as HTMLElement).style.animationDuration).toBe('1000ms');

    act(() => vi.advanceTimersByTime(1000));
    expect(item?.getAttribute('data-toast-phase')).toBe('exiting');
    expect(screen.getByText('Укажите название проекта')).not.toBeNull();

    act(() => vi.advanceTimersByTime(TOAST_EXIT_DURATION));
    expect(screen.queryByText('Укажите название проекта')).toBeNull();
  });

  it('dismisses a toast when it is dragged far enough to the right', () => {
    render(<Toaster />);
    act(() => {
      toast.default('Перетащите меня', { duration: 0 });
    });

    const item = screen.getByText('Перетащите меня').closest('[data-toast-phase]') as HTMLElement;
    fireEvent.pointerDown(item, { pointerId: 1, clientX: 20 });
    fireEvent.pointerMove(item, { pointerId: 1, clientX: 130 });
    fireEvent.pointerUp(item, { pointerId: 1, clientX: 130 });

    expect(item.getAttribute('data-toast-phase')).toBe('exiting');
    expect(item.style.transform).toContain('translate3d');

    act(() => vi.advanceTimersByTime(TOAST_EXIT_DURATION));
    expect(screen.queryByText('Перетащите меня')).toBeNull();
  });
});
