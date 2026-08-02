import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, within } from 'storybook/test';
import { Tabs, TabsList, TabsTrigger, TabsContent } from './Tabs';

const meta = {
  title: 'Shared UI/Tabs',
  component: Tabs,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof Tabs>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultTabs: Story = {
  render: () => (
    <Tabs defaultValue="transcript" className="w-[400px]">
      <TabsList>
        <TabsTrigger value="transcript">Transcript</TabsTrigger>
        <TabsTrigger value="media">Media</TabsTrigger>
        <TabsTrigger value="jobs">Jobs</TabsTrigger>
      </TabsList>
      <TabsContent value="transcript">Read-only transcript</TabsContent>
      <TabsContent value="media">Source metadata</TabsContent>
      <TabsContent value="jobs">Operation history</TabsContent>
    </Tabs>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const transcriptTab = canvas.getByRole('tab', { name: 'Transcript' });
    const mediaTab = canvas.getByRole('tab', { name: 'Media' });
    const transcriptPanel = canvas.getByRole('tabpanel', { name: 'Transcript' });

    await expect(transcriptTab).toHaveAttribute('aria-controls', transcriptPanel.id);
    await transcriptTab.focus();
    await userEvent.keyboard('{ArrowRight}');
    const mediaPanel = canvas.getByRole('tabpanel', { name: 'Media' });

    await expect(mediaTab).toHaveFocus();
    await expect(mediaTab).toHaveAttribute('aria-selected', 'true');
    await expect(mediaPanel).toBeVisible();
    await expect(mediaPanel).toHaveAttribute('aria-labelledby', mediaTab.id);
  },
};

export const CompactTabs: Story = {
  render: () => (
    <Tabs defaultValue="all" variant="compact" className="w-[400px]">
      <TabsList>
        <TabsTrigger value="all">All Files</TabsTrigger>
        <TabsTrigger value="videos">Videos</TabsTrigger>
        <TabsTrigger value="audio">Audio</TabsTrigger>
      </TabsList>
      <TabsContent value="all">All imported files</TabsContent>
      <TabsContent value="videos">Video files</TabsContent>
      <TabsContent value="audio">Audio files</TabsContent>
    </Tabs>
  ),
};

export const FullWidthTabs: Story = {
  render: () => (
    <Tabs defaultValue="tab1" fullWidth className="w-[500px]">
      <TabsList>
        <TabsTrigger value="tab1">Left</TabsTrigger>
        <TabsTrigger value="tab2">Center</TabsTrigger>
        <TabsTrigger value="tab3">Right</TabsTrigger>
      </TabsList>
      <TabsContent value="tab1">Left panel</TabsContent>
      <TabsContent value="tab2">Center panel</TabsContent>
      <TabsContent value="tab3">Right panel</TabsContent>
    </Tabs>
  ),
};

export const DisabledTab: Story = {
  render: () => (
    <Tabs defaultValue="transcript" className="w-[400px]">
      <TabsList>
        <TabsTrigger value="transcript">Transcript</TabsTrigger>
        <TabsTrigger value="media">Media</TabsTrigger>
        <TabsTrigger value="export" disabled>
          Export unavailable
        </TabsTrigger>
      </TabsList>
      <TabsContent value="transcript">Read-only transcript</TabsContent>
      <TabsContent value="media">Source metadata</TabsContent>
      <TabsContent value="export">Export is not available in this version.</TabsContent>
    </Tabs>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const disabledTab = canvas.getByRole('tab', { name: 'Export unavailable' });

    await expect(disabledTab).toBeDisabled();
    await expect(disabledTab).toHaveAttribute('aria-selected', 'false');
  },
};

export const TabsWithContent: Story = {
  render: () => (
    <Tabs defaultValue="general" className="w-[400px]">
      <TabsList fullWidth>
        <TabsTrigger value="general">General</TabsTrigger>
        <TabsTrigger value="advanced">Advanced</TabsTrigger>
      </TabsList>
      <TabsContent
        value="general"
        className="p-4 border border-muted/20 rounded-md mt-4 bg-surface"
      >
        <h3 className="text-lg font-medium mb-2">General Settings</h3>
        <p className="text-sm text-muted">Update your main preferences here.</p>
      </TabsContent>
      <TabsContent
        value="advanced"
        className="p-4 border border-muted/20 rounded-md mt-4 bg-surface"
      >
        <h3 className="text-lg font-medium mb-2">Advanced Settings</h3>
        <p className="text-sm text-muted">Danger zone! Be careful with what you change here.</p>
      </TabsContent>
    </Tabs>
  ),
};
