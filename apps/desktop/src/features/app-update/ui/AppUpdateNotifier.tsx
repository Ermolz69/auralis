import { useEffect, useRef } from 'react';
import { toast } from '../../../shared/ui/toast';
import { useAppUpdate } from '../model/appUpdateContext';

export function AppUpdateNotifier({ onOpen }: { onOpen: () => void }) {
  const { phase, update } = useAppUpdate();
  const announcedVersion = useRef<string | null>(null);

  useEffect(() => {
    if (phase !== 'available' || !update || announcedVersion.current === update.version) return;
    announcedVersion.current = update.version;
    toast.default(`Auralis ${update.version} is available`, {
      description: 'The update is signed and ready to install from GitHub Releases.',
      duration: 12_000,
      action: { label: 'View update', onClick: onOpen },
    });
  }, [onOpen, phase, update]);

  return null;
}
