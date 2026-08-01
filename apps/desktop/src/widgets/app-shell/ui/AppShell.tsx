import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useProjectContext } from '@/entities/project';
import { useNavigation, type View } from '@/shared/router';
import { Button } from '@/shared/ui/button';
import { Icon } from '@/shared/ui/icon';

const locationLabel: Record<View, string> = {
  home: 'Projects',
  project: 'Workspace',
  settings: 'Settings',
};

export function AppShell({ children }: { children: ReactNode }) {
  const { currentView, setCurrentView } = useNavigation();
  const { projectId } = useProjectContext();
  const mainRef = useRef<HTMLElement>(null);
  const [settingsReturnView, setSettingsReturnView] = useState<View>('home');

  useEffect(() => {
    mainRef.current?.focus();
  }, [currentView]);

  useEffect(() => {
    if (!projectId && settingsReturnView === 'project') {
      setSettingsReturnView('home');
    }
  }, [projectId, settingsReturnView]);

  const goToHome = () => setCurrentView('home');

  const goToProject = () => {
    if (projectId) {
      setCurrentView('project');
    }
  };

  const goToSettings = () => {
    setSettingsReturnView(currentView === 'project' && projectId ? 'project' : 'home');
    setCurrentView('settings');
  };

  const returnFromSettings = () => {
    setCurrentView(settingsReturnView === 'project' && projectId ? 'project' : 'home');
  };

  return (
    <div className="min-h-screen bg-bg text-text flex flex-col">
      <header className="shrink-0 border-b border-muted/30 bg-surface/95 px-4 py-3 shadow-sm">
        <div className="mx-auto flex w-full max-w-7xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex min-w-0 items-center gap-3">
            {currentView === 'settings' && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={returnFromSettings}
                leftIcon={<Icon name="ArrowLeft" size="sm" />}
              >
                Back
              </Button>
            )}
            <div className="min-w-0">
              <p className="text-sm font-semibold leading-none text-text">Auralis</p>
              <p className="mt-1 truncate text-xs text-muted">{locationLabel[currentView]}</p>
            </div>
          </div>

          <nav aria-label="Primary" className="flex flex-wrap items-center gap-2">
            <ShellDestination
              active={currentView === 'home'}
              icon="Library"
              label="Projects"
              onClick={goToHome}
            />
            <ShellDestination
              active={currentView === 'project'}
              disabled={!projectId}
              icon="MonitorPlay"
              label="Workspace"
              onClick={goToProject}
            />
            <ShellDestination
              active={currentView === 'settings'}
              icon="Settings"
              label="Settings"
              onClick={goToSettings}
            />
          </nav>
        </div>
      </header>

      <main
        ref={mainRef}
        tabIndex={-1}
        aria-label={locationLabel[currentView]}
        className="min-h-0 flex-1 outline-none"
      >
        {children}
      </main>
    </div>
  );
}

type ShellDestinationProps = {
  active: boolean;
  disabled?: boolean;
  icon: 'Library' | 'MonitorPlay' | 'Settings';
  label: string;
  onClick: () => void;
};

function ShellDestination({ active, disabled, icon, label, onClick }: ShellDestinationProps) {
  return (
    <Button
      type="button"
      variant={active ? 'secondary' : 'ghost'}
      size="sm"
      disabled={disabled}
      aria-current={active ? 'page' : undefined}
      aria-label={disabled ? `${label} unavailable without an active project` : label}
      title={disabled ? 'Open a project first' : undefined}
      onClick={onClick}
      leftIcon={<Icon name={icon} size="sm" />}
      className="min-w-28"
    >
      {label}
    </Button>
  );
}
