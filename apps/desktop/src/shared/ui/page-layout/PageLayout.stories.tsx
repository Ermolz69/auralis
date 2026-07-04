import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import {
  Page,
  PageContainer,
  PageHeader,
  PageHeaderGroup,
  PageTitle,
  PageDescription,
  PageActions,
  PageContent,
  PageLayoutWithSidebar,
  PageSidebar,
  PageSidebarContent,
} from './PageLayout';
import { Button } from '../button';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../card';
import { Icon } from '../icon';

const meta = {
  title: 'UI Kit/PageLayout',
  component: Page,
  parameters: {
    layout: 'fullscreen',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof Page>;

export default meta;
type Story = StoryObj<typeof meta>;

export const BasicPage: Story = {
  render: () => (
    <Page>
      <PageContainer>
        <PageHeader>
          <PageHeaderGroup>
            <PageTitle>Settings</PageTitle>
            <PageDescription>Manage your account settings and set email preferences.</PageDescription>
          </PageHeaderGroup>
        </PageHeader>
        <PageContent>
          <div className="h-64 rounded-xl border border-dashed border-muted/50 flex items-center justify-center text-muted">
            Content goes here
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  ),
};

export const PageWithActions: Story = {
  render: () => (
    <Page>
      <PageContainer>
        <PageHeader>
          <PageHeaderGroup>
            <PageTitle>Projects</PageTitle>
            <PageDescription>View and manage your AI dubbing projects.</PageDescription>
          </PageHeaderGroup>
          <PageActions>
            <Button variant="secondary" leftIcon={<Icon name="Download" size="sm" />}>Export All</Button>
            <Button leftIcon={<Icon name="Plus" size="sm" />}>New Project</Button>
          </PageActions>
        </PageHeader>
        <PageContent>
          <div className="h-64 rounded-xl border border-dashed border-muted/50 flex items-center justify-center text-muted">
            Project List
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  ),
};

export const PageWithCards: Story = {
  render: () => (
    <Page>
      <PageContainer>
        <PageHeader>
          <PageHeaderGroup>
            <PageTitle>Dashboard</PageTitle>
            <PageDescription>Overview of your activity and usage.</PageDescription>
          </PageHeaderGroup>
        </PageHeader>
        <PageContent>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <Card>
              <CardHeader>
                <CardTitle>Total Minutes</CardTitle>
                <CardDescription>Dubbed this month</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-bold">1,240</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Active Projects</CardTitle>
                <CardDescription>Currently processing</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-bold">3</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>API Calls</CardTitle>
                <CardDescription>Usage limit</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-bold">42%</div>
              </CardContent>
            </Card>
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  ),
};

export const DenseProjectPage: Story = {
  render: () => (
    <Page>
      <PageContainer size="full">
        <PageHeader>
          <PageHeaderGroup>
            <div className="flex items-center gap-2">
              <Button variant="ghost" className="px-2" aria-label="Back">
                <Icon name="ArrowLeft" />
              </Button>
              <PageTitle>My Awesome Video</PageTitle>
            </div>
            <PageDescription>Status: Processing audio...</PageDescription>
          </PageHeaderGroup>
          <PageActions>
            <Button variant="danger">Cancel Job</Button>
          </PageActions>
        </PageHeader>
        <PageContent>
          <PageLayoutWithSidebar>
            <PageSidebarContent>
              <div className="h-96 rounded-xl border border-dashed border-muted/50 flex items-center justify-center text-muted bg-surface/50">
                Video Player Area
              </div>
            </PageSidebarContent>
            <PageSidebar>
              <Card className="h-full">
                <CardHeader>
                  <CardTitle>Timeline</CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted">Subtitles and segments will appear here.</p>
                </CardContent>
              </Card>
            </PageSidebar>
          </PageLayoutWithSidebar>
        </PageContent>
      </PageContainer>
    </Page>
  ),
};

export const EmptyStatePage: Story = {
  render: () => (
    <Page>
      <PageContainer>
        <PageContent className="items-center justify-center">
          <div className="flex flex-col items-center max-w-md text-center gap-6 m-auto">
            <div className="w-16 h-16 rounded-full bg-surface flex items-center justify-center shadow-inner">
              <Icon name="FolderOpen" size={32} color="muted" />
            </div>
            <div className="flex flex-col gap-2">
              <h2 className="text-2xl font-bold">No projects yet</h2>
              <p className="text-muted">Create your first AI dubbing project to get started. It only takes a minute to upload a video or paste a YouTube link.</p>
            </div>
            <Button leftIcon={<Icon name="Plus" size="sm" />}>Create Project</Button>
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  ),
};
