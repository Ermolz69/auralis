// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { createWindowEventChannel } from './windowEventChannel';

describe('createWindowEventChannel', () => {
  it('delivers typed details and stops after unsubscribe', () => {
    const channel = createWindowEventChannel<{ id: string }>('auralis:test-channel');
    const listener = vi.fn();
    const unsubscribe = channel.subscribe(listener);

    channel.emit({ id: 'p1' });
    expect(listener).toHaveBeenCalledWith({ id: 'p1' });

    unsubscribe();
    channel.emit({ id: 'p2' });
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it('supports independent subscribers and removes only the requested listener', () => {
    const channel = createWindowEventChannel<number>('auralis:test-multiple-subscribers');
    const first = vi.fn();
    const second = vi.fn();
    const unsubscribeFirst = channel.subscribe(first);
    const unsubscribeSecond = channel.subscribe(second);

    channel.emit(1);
    unsubscribeFirst();
    channel.emit(2);

    expect(first).toHaveBeenCalledExactlyOnceWith(1);
    expect(second).toHaveBeenNthCalledWith(1, 1);
    expect(second).toHaveBeenNthCalledWith(2, 2);

    unsubscribeSecond();
  });

  it('isolates channels with different event names', () => {
    const firstChannel = createWindowEventChannel<string>('auralis:test-first-channel');
    const secondChannel = createWindowEventChannel<string>('auralis:test-second-channel');
    const firstListener = vi.fn();
    const secondListener = vi.fn();
    const unsubscribeFirst = firstChannel.subscribe(firstListener);
    const unsubscribeSecond = secondChannel.subscribe(secondListener);

    firstChannel.emit('first');

    expect(firstListener).toHaveBeenCalledExactlyOnceWith('first');
    expect(secondListener).not.toHaveBeenCalled();

    unsubscribeFirst();
    unsubscribeSecond();
  });

  it('supports signal-only channels with a void payload', () => {
    const channel = createWindowEventChannel<void>('auralis:test-signal-channel');
    const listener = vi.fn();
    const unsubscribe = channel.subscribe(listener);

    channel.emit();

    expect(listener).toHaveBeenCalledOnce();
    unsubscribe();
  });
});
