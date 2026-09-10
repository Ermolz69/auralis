// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import {
  Badge,
  Button,
  Card,
  Icon,
  Notice,
  Progress,
  type BadgeProps,
  type ButtonProps,
  type CardProps,
  type IconProps,
  type NoticeProps,
  type ProgressProps,
} from '@/shared/ui';

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('Storybook-facing component boundaries', () => {
  it('falls back to stable styles when an out-of-contract variant reaches a component', () => {
    render(
      <>
        <Button variant={'unknown' as ButtonProps['variant']} size={'huge' as ButtonProps['size']}>
          Safe button
        </Button>
        <Badge variant={'unknown' as BadgeProps['variant']} size={'huge' as BadgeProps['size']}>
          Safe badge
        </Badge>
        <Card data-testid="safe-card" variant={'unknown' as CardProps['variant']}>
          Safe card
        </Card>
        <Notice icon="Info" tone={'unknown' as NoticeProps['tone']}>
          Safe notice
        </Notice>
        <Progress
          value={50}
          label="Safe progress"
          variant={'unknown' as ProgressProps['variant']}
        />
      </>,
    );

    expect(screen.getByRole('button', { name: 'Safe button' }).className).toContain(
      'bg-primary-action',
    );
    expect(screen.getByText('Safe badge').parentElement?.className).toContain('bg-surface-hover');
    expect(screen.getByTestId('safe-card').className).toContain('bg-surface-raised');
    expect(screen.getByText('Safe notice').closest('div.flex')?.className).toContain(
      'border-muted/40',
    );
    expect(screen.getByRole('progressbar', { name: 'Safe progress' }).innerHTML).toContain(
      'bg-primary',
    );
  });

  it('normalizes non-finite progress values instead of emitting invalid accessibility state', () => {
    render(<Progress value={Number.NaN} max={Number.POSITIVE_INFINITY} label="Import" />);

    const progress = screen.getByRole('progressbar', { name: 'Import' });
    expect(progress.getAttribute('aria-valuemax')).toBe('100');
    expect(progress.getAttribute('aria-valuenow')).toBe('0');
    expect(progress.getAttribute('aria-valuetext')).toBe('0%');
    expect(progress.firstElementChild?.getAttribute('style')).toContain('translateX(-100%)');
  });

  it('renders a visible fallback when an unknown icon name bypasses Storybook controls', () => {
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => undefined);

    render(<Icon name={'NotRegistered' as IconProps['name']} ariaLabel="Unknown icon fallback" />);

    expect(screen.getByRole('img', { name: 'Unknown icon fallback' })).not.toBeNull();
    expect(warning).toHaveBeenCalledOnce();
  });
});
