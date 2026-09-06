import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { COLOR_THEMES } from '@/shared/theme/config/colorThemes';
import { Badge } from '@/shared/ui/badge';
import { Button } from '@/shared/ui/button';

function ThemeGallery() {
  return (
    <div
      className="grid gap-5 xl:grid-cols-2"
      aria-label="Auralis color theme gallery"
      role="region"
    >
      {COLOR_THEMES.map((theme) => (
        <article
          key={theme.id}
          data-color-theme={theme.id}
          className="overflow-hidden rounded-xl border border-border bg-bg text-text shadow-md"
        >
          <div className="flex items-start justify-between gap-4 border-b border-border bg-surface p-5">
            <div className="space-y-1">
              <h2 className="text-lg font-bold">{theme.label}</h2>
              <p className="text-sm text-muted">{theme.description}</p>
            </div>
            <Badge variant={theme.appearance === 'dark' ? 'primary' : 'accent'}>
              {theme.appearance}
            </Badge>
          </div>

          <div className="space-y-5 p-5">
            <div className="grid grid-cols-6 gap-2" aria-label={`${theme.label} semantic colors`}>
              {['primary', 'accent', 'success', 'warning', 'danger', 'surface'].map((token) => (
                <div key={token} className="space-y-1 text-center">
                  <div
                    className="h-10 rounded-md border border-border"
                    style={{ backgroundColor: `var(--color-${token})` }}
                  />
                  <span className="text-[0.625rem] text-muted">{token}</span>
                </div>
              ))}
            </div>

            <div className="rounded-lg border border-border bg-surface-raised p-4">
              <p className="font-semibold">Source is ready</p>
              <p className="mt-1 text-sm text-muted">
                Review the transcript before starting the next operation.
              </p>
              <div className="mt-4 flex flex-wrap gap-2">
                <Button size="sm">Open project</Button>
                <Button size="sm" variant="secondary">
                  View details
                </Button>
              </div>
            </div>
          </div>
        </article>
      ))}
    </div>
  );
}

const meta = {
  title: 'Design System/Foundations/Theme Gallery',
  component: ThemeGallery,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Side-by-side visual review of every supported application palette using the same semantic tokens and product composition.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof ThemeGallery>;

export default meta;
type Story = StoryObj<typeof meta>;

export const AllThemes: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const gallery = canvas.getByRole('region', { name: 'Auralis color theme gallery' });

    await expect(within(gallery).getAllByRole('article')).toHaveLength(COLOR_THEMES.length);
    for (const theme of COLOR_THEMES) {
      await expect(canvas.getByRole('heading', { name: theme.label })).toBeInTheDocument();
    }
  },
};
