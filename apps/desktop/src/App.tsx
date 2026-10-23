import { AppUpdateNotifier, AppUpdateProvider } from './features/app-update';
import { LibraryProvider, useLibrary } from './entities/library';
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
  const { setView } = useLibrary();
  return (
    <>
      <AppUpdateNotifier onOpen={() => setView('settings')} />
      <StudioShell />
    </>
  );
}
