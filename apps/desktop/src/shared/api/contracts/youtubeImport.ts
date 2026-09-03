export type PendingYoutubeImport = {
  projectId: string;
  title: string;
  state: 'Downloading' | 'Staged' | 'Failed';
};
