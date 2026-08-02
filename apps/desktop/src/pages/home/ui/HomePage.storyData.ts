export const projects = [
  {
    title: 'YouTube project',
    status: 'Processing',
    sourceLabel: 'YouTube source (youtube.com)',
  },
  {
    title: 'local-interview.mp4',
    status: 'Source imported',
    sourceLabel: 'local-interview.mp4',
  },
];

export type HomeProject = (typeof projects)[number];
