import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from '@/shared/ui/button';
import { Input } from '@/shared/ui/input';

const meta = {
  title: 'Features/PasteYoutubeLink/States',
  parameters: { layout: 'centered' },
  tags: ['autodocs'],
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  render: () => <YoutubeFormScenario />,
};

export const CreatingProject: Story = {
  render: () => <YoutubeFormScenario value="https://youtube.com/watch?v=abc" loading />,
};

export const InlineError: Story = {
  render: () => (
    <YoutubeFormScenario
      value="https://youtube.com/watch?v=abc"
      error="Could not create project. Check the link and try again."
    />
  ),
};

export const SuccessHandoff: Story = {
  render: () => (
    <div className="w-[36rem] space-y-3 text-left">
      <YoutubeFormScenario />
      <p className="text-sm text-success" role="status">
        Project created. The workspace opens and the subtitle import job continues there.
      </p>
    </div>
  ),
};

function YoutubeFormScenario({
  value = '',
  loading = false,
  error,
}: {
  value?: string;
  loading?: boolean;
  error?: string;
}) {
  return (
    <form className="flex w-[36rem] flex-col gap-3 text-left" aria-label="Add YouTube source">
      <div className="flex gap-2">
        <Input
          label="YouTube URL"
          helperText="Supported source: a single YouTube video link."
          error={Boolean(error)}
          errorText={error}
          placeholder="https://youtube.com/watch?v=..."
          defaultValue={value}
          className="flex-1"
        />
        <Button
          type="submit"
          variant="secondary"
          size="lg"
          loading={loading}
          disabled={loading || !value}
        >
          Add from YouTube
        </Button>
      </div>
      {loading && (
        <p className="text-muted text-sm" role="status" aria-live="polite">
          Creating project. The workspace will show the running subtitle import job after handoff.
        </p>
      )}
    </form>
  );
}
