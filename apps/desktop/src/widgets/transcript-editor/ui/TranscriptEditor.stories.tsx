import type { Meta, StoryObj } from '@storybook/react-vite';
import { TranscriptEditor } from './TranscriptEditor';
import { ProjectContext } from '@/entities/project';
import type { ProjectContextType } from '@/entities/project';
import type { Project } from '@/shared/api/contracts/project';

const meta = {
  title: 'Widgets/TranscriptEditor',
  component: TranscriptEditor,
  parameters: {
    layout: 'padded',
  },
  decorators: [
    (Story, context) => {
      // Provide a mock context
      const mockProject: Project = context.args.project || {
        id: '1',
        title: 'Test',
        status: 'processing',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        source: {
          kind: 'youtubeUrl',
          urlOrPath: 'https://youtube.com/watch?v=123',
        },
      };

      const mockContext: ProjectContextType = {
        projectId: mockProject.id,
        setProjectId: () => {},
        project: mockProject,
        setProject: () => {},
        currentView: 'project',
        setCurrentView: () => {},
      };

      return (
        <ProjectContext.Provider value={mockContext}>
          <div className="h-[600px] flex">
            <Story />
          </div>
        </ProjectContext.Provider>
      );
    },
  ],
} satisfies Meta<typeof TranscriptEditor>;

export default meta;
type Story = StoryObj<typeof meta>;

export const LocalMediaUnavailable: Story = {
  args: {
    project: {
      id: '2',
      title: 'Local Test',
      status: 'completed',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      source: {
        kind: 'localFile',
        urlOrPath: '/path/to/local/file.mp4',
      },
    },
  } as any,
  parameters: {
    mockData: [
      {
        url: 'api://transcript',
        method: 'GET',
        status: 200,
        response: { segments: [] },
      },
    ],
  },
};

