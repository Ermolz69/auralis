import type { Meta, StoryObj } from '@storybook/react-vite';
import { Tabs, TabsList, TabsTrigger, TabsContent } from './Tabs';

import { ControlledTabs, PersistentPanels, VerticalPanels } from './Tabs.storyFixtures';
import {
  playDefaultTabs,
  playDisabledTab,
  playControlledSelection,
  playRetainedPanelState,
  playVerticalKeyboard,
} from './Tabs.interactions.storyFixtures';

const meta = {
  title: 'Design System/Components/Tabs',
  component: Tabs,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Keyboard-accessible tabs for switching between closely related workspace views.',
      },
    },
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

  play: playDefaultTabs,
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

  play: playDisabledTab,
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

export const ControlledSelection: Story = {
  render: () => <ControlledTabs />,
  play: playControlledSelection,
};

export const RetainedPanelState: Story = {
  render: () => <PersistentPanels />,
  play: playRetainedPanelState,
};

export const VerticalKeyboard: Story = {
  render: () => <VerticalPanels />,
  play: playVerticalKeyboard,
};
