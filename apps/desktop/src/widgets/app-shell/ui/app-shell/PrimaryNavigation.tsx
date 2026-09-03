import type { View } from '@/shared/router';
import { ShellDestination } from './ShellDestination';

type Props = {
  currentView: View;
  hasProject: boolean;
  onHome: () => void;
  onProject: () => void;
  onSettings: () => void;
};

export function PrimaryNavigation(props: Props) {
  return (
    <nav
      aria-label="Primary"
      className="grid h-full w-full grid-cols-3 items-stretch lg:h-auto lg:grid-cols-1 lg:border-t lg:border-border lg:py-1.5"
    >
      <ShellDestination
        active={props.currentView === 'home'}
        icon="LayoutGrid"
        label="Projects"
        visualLabel="Проекты"
        onClick={props.onHome}
      />
      <ShellDestination
        active={props.currentView === 'project'}
        disabled={!props.hasProject}
        icon="MonitorPlay"
        label="Workspace"
        visualLabel="Рабочее пространство"
        onClick={props.onProject}
        desktopHidden
      />
      <ShellDestination
        active={props.currentView === 'settings'}
        icon="Settings"
        label="Settings"
        visualLabel="Настройки"
        onClick={props.onSettings}
      />
    </nav>
  );
}
