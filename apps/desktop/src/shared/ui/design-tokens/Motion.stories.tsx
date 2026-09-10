import type { Meta, StoryObj } from '@storybook/react-vite';
import { Badge } from '../badge';

const motionTokens = [
  {
    token: '--motion-duration-fast',
    value: '140 ms',
    usage: 'Hover, focus, compact popovers, and immediate feedback',
  },
  {
    token: '--motion-duration-base',
    value: '180 ms',
    usage: 'Content changes and standard control transitions',
  },
  {
    token: '--motion-duration-reveal',
    value: '220 ms',
    usage: 'Dialogs, drawers, and newly revealed surfaces',
  },
] as const;

function MotionReference() {
  return (
    <div className="mx-auto max-w-4xl space-y-8 text-text">
      <header className="space-y-2">
        <Badge variant="primary">Motion foundation</Badge>
        <h1 className="text-3xl font-bold">Short, purposeful, interruptible</h1>
        <p className="max-w-2xl text-muted">
          Motion explains hierarchy and state change. It never delays input, and non-essential
          transforms are disabled when the operating system requests reduced motion.
        </p>
      </header>

      <div className="grid gap-4 md:grid-cols-3">
        {motionTokens.map((item, index) => (
          <article
            key={item.token}
            className="animate-surface-in rounded-xl border border-border bg-surface p-5 shadow-sm"
            style={{ animationDelay: `${index * 45}ms` }}
          >
            <p className="font-mono text-xs text-primary">{item.token}</p>
            <p className="mt-3 text-2xl font-bold">{item.value}</p>
            <p className="mt-2 text-sm text-muted">{item.usage}</p>
          </article>
        ))}
      </div>

      <section className="rounded-xl border border-border bg-surface p-6">
        <h2 className="text-lg font-bold">Rules</h2>
        <ul className="mt-3 list-disc space-y-2 pl-5 text-sm text-muted">
          <li>Use opacity and transform for entrance motion; avoid layout-shifting properties.</li>
          <li>Use the standard easing for direct manipulation and the enter easing for reveals.</li>
          <li>Only progress indicators may loop, and only while work is genuinely active.</li>
          <li>Never encode status or completion using motion alone.</li>
        </ul>
      </section>
    </div>
  );
}

const meta = {
  title: 'Design System/Foundations/Motion',
  component: MotionReference,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Motion durations, intended uses, and accessibility rules shared by all Auralis interactions.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof MotionReference>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Reference: Story = {};
