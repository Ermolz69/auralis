export type View = 'home' | 'project' | 'settings';

export interface NavigationContextType {
  currentView: View;
  setCurrentView: (view: View) => void;
}
