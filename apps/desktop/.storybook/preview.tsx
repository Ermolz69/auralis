import type { Preview } from '@storybook/react-vite';
import { mockIPC } from '@tauri-apps/api/mocks';
import '../src/app/styles/index.css';

mockIPC(async (cmd, payload) => {
  if (cmd === 'get_transcript_cmd') {
    const projectId = payload?.projectId;

    if (projectId === 'local-unavailable') {
      return {
        language: 'en',
        segments: [],
      };
    }

    if (projectId === 'youtube-ready') {
      return {
        language: 'en',
        segments: [
          {
            id: 'segment-1',
            index: 0,
            startMs: 1000,
            endMs: 4200,
            sourceText: 'Welcome to the product walkthrough.',
          },
          {
            id: 'segment-2',
            index: 1,
            startMs: 4400,
            endMs: 7800,
            sourceText: 'The transcript is available as read-only text.',
          },
        ],
      };
    }

    return null;
  }

  if (cmd === 'plugin:event|listen') return null;
  if (cmd === 'plugin:event|unlisten') return null;

  return null;
});

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },

    a11y: {
      test: 'error',
    },
  },
};

export default preview;
