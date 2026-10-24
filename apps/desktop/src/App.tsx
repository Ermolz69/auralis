import { AppUpdateNotifier, AppUpdateProvider } from './features/app-update';
import { LibraryProvider } from './entities/library';
import { StudioShell } from './app/studio';

export default function App() {
  return (
    <AppUpdateProvider>
      <LibraryProvider>
        <Studio />
      </LibraryProvider>
    </AppUpdateProvider>
  );
}
function Studio() {
  return (
    <>
      <AppUpdateNotifier onOpen={() => window.dispatchEvent(new Event('auralis-open-settings'))} />
      <StudioShell />
    </>
  );
}
