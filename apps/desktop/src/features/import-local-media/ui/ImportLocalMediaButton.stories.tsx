import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from '@/shared/ui/button';
import { Card, CardContent } from '@/shared/ui/card';
import { Icon } from '@/shared/ui/icon';

const meta = {
  title: 'Features/ImportLocalMedia/States',
  parameters: { layout: 'centered' },
  tags: ['autodocs'],
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const Idle: Story = {
  render: () => <ImportLocalMediaScenario />,
};

export const Selecting: Story = {
  render: () => <ImportLocalMediaScenario status="Waiting for file selection" loading />,
};

export const CheckingMedia: Story = {
  render: () => (
    <ImportLocalMediaScenario status="Checking media: long-local-recording-name.mp4" loading />
  ),
};

export const Importing: Story = {
  render: () => (
    <ImportLocalMediaScenario
      status="Importing into project: long-local-recording-name.mp4"
      loading
    />
  ),
};

export const DraftRecovery: Story = {
  render: () => <ImportLocalMediaScenario error draftSaved />,
};

function ImportLocalMediaScenario({
  status,
  loading = false,
  error = false,
  draftSaved = false,
}: {
  status?: string;
  loading?: boolean;
  error?: boolean;
  draftSaved?: boolean;
}) {
  return (
    <div className="flex w-96 flex-col items-stretch gap-3">
      <Button
        loading={loading}
        disabled={loading}
        size="lg"
        leftIcon={<Icon name="Film" size="sm" />}
        fullWidth
      >
        Import local video
      </Button>
      {status && (
        <p className="text-muted text-sm" role="status" aria-live="polite">
          {status}
        </p>
      )}
      {error && <DraftRecoveryCard draftSaved={draftSaved} />}
    </div>
  );
}

function DraftRecoveryCard({ draftSaved }: { draftSaved: boolean }) {
  return (
    <Card variant="muted" className="border-danger/40 text-left" role="alert" tabIndex={-1}>
      <CardContent className="p-4 flex flex-col gap-3">
        <div>
          <p className="text-sm font-semibold text-text">Local import did not finish</p>
          <p className="text-danger text-sm">Import storage failed</p>
          <p className="text-muted text-sm mt-1">
            {draftSaved
              ? 'A draft project was saved. You can open it, choose the file again, or delete the draft from Recent Projects.'
              : 'No project was created. Choose the file again when ready.'}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="secondary" size="sm">
            Choose file again
          </Button>
          {draftSaved && (
            <Button type="button" variant="ghost" size="sm">
              Open draft
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
