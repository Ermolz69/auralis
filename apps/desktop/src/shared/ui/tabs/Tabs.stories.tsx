import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { Tabs, TabsList, TabsTrigger, TabsContent } from './Tabs';

const meta = {
  title: 'UI Kit/Tabs',
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
    <Tabs defaultValue="account" className="w-[400px]">
      <TabsList>
        <TabsTrigger value="account">Account</TabsTrigger>
        <TabsTrigger value="password">Password</TabsTrigger>
        <TabsTrigger value="settings">Settings</TabsTrigger>
      </TabsList>
    </Tabs>
  ),
};

export const CompactTabs: Story = {
  render: () => (
    <Tabs defaultValue="all" variant="compact" className="w-[400px]">
      <TabsList>
        <TabsTrigger value="all">All Files</TabsTrigger>
        <TabsTrigger value="videos">Videos</TabsTrigger>
        <TabsTrigger value="audio">Audio</TabsTrigger>
      </TabsList>
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
    </Tabs>
  ),
};

export const DisabledTab: Story = {
  render: () => (
    <Tabs defaultValue="free" className="w-[400px]">
      <TabsList>
        <TabsTrigger value="free">Free Plan</TabsTrigger>
        <TabsTrigger value="pro">Pro Plan</TabsTrigger>
        <TabsTrigger value="enterprise" disabled>
          Enterprise (Coming Soon)
        </TabsTrigger>
      </TabsList>
    </Tabs>
  ),
};

export const TabsWithContent: Story = {
  render: () => (
    <Tabs defaultValue="general" className="w-[400px]">
      <TabsList fullWidth>
        <TabsTrigger value="general">General</TabsTrigger>
        <TabsTrigger value="advanced">Advanced</TabsTrigger>
      </TabsList>
      <TabsContent value="general" className="p-4 border border-muted/20 rounded-md mt-4 bg-surface">
        <h3 className="text-lg font-medium mb-2">General Settings</h3>
        <p className="text-sm text-muted">Update your main preferences here.</p>
      </TabsContent>
      <TabsContent value="advanced" className="p-4 border border-muted/20 rounded-md mt-4 bg-surface">
        <h3 className="text-lg font-medium mb-2">Advanced Settings</h3>
        <p className="text-sm text-muted">Danger zone! Be careful with what you change here.</p>
      </TabsContent>
    </Tabs>
  ),
};
