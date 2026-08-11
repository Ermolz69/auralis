export type View = 'home' | 'project' | 'settings';
export type PipelineStep = 'source' | 'subtitles';

export interface NavigationContextType {
  currentView: View;
  setCurrentView: (view: View) => void;
  pipelineStep: PipelineStep;
  setPipelineStep: (step: PipelineStep) => void;
}
