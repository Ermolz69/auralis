import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from './Card';
import { Button } from '../button';

const meta = {
  title: 'UI Kit/Card',
  component: Card,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
  argTypes: {
    variant: { control: 'select', options: ['default', 'elevated', 'interactive', 'muted'] },
  },
} satisfies Meta<typeof Card>;

export default meta;
type Story = StoryObj<typeof meta>;

// Default
export const Default: Story = {
  args: { variant: 'default' },
  render: (args) => (
    <Card className="w-[350px]" {...args}>
      <CardHeader>
        <CardTitle>Project Settings</CardTitle>
        <CardDescription>Manage your project configuration.</CardDescription>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted">Form content goes here...</p>
      </CardContent>
    </Card>
  ),
};

// With Footer Actions
export const CardWithFooterActions: Story = {
  render: () => (
    <Card className="w-[350px]">
      <CardHeader>
        <CardTitle>Export Video</CardTitle>
        <CardDescription>Save the dubbed video to your computer.</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-2">
          <span className="text-sm">Format: MP4</span>
          <span className="text-sm">Quality: 1080p</span>
        </div>
      </CardContent>
      <CardFooter className="flex justify-between">
        <Button variant="ghost">Cancel</Button>
        <Button>Export</Button>
      </CardFooter>
    </Card>
  ),
};

// Interactive
export const InteractiveCard: Story = {
  render: () => (
    <Card className="w-[350px]" variant="interactive">
      <CardHeader>
        <CardTitle>Clickable Card</CardTitle>
        <CardDescription>Hover over me to see the effect.</CardDescription>
      </CardHeader>
      <CardContent>
        <p className="text-sm">This is useful for selecting items from a grid.</p>
      </CardContent>
    </Card>
  ),
};

// Dense / Muted
export const DenseCard: Story = {
  render: () => (
    <Card className="w-[300px]" variant="muted">
      <CardHeader className="p-4">
        <CardTitle className="text-base">Subtle & Dense</CardTitle>
      </CardHeader>
      <CardContent className="p-4 pt-0">
        <p className="text-xs">Less padding, muted background.</p>
      </CardContent>
    </Card>
  ),
};
