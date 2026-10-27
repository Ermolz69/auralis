import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { describeCommandError } from '@/shared/api/contracts/error';

export function useWindowChrome() {
  const [nativeWindow] = useState(() => (isTauri() ? getCurrentWindow() : null));
  const [maximized, setMaximized] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    if (!nativeWindow) return;
    let active = true;
    const sync = async () => {
      try {
        const value = await nativeWindow.isMaximized();
        if (active) setMaximized(value);
      } catch (reason) {
        if (active) setError(describeCommandError(reason));
      }
    };
    void sync();
    const subscription = nativeWindow.onResized(() => void sync());
    void subscription.catch((reason) => {
      if (active) setError(describeCommandError(reason));
    });
    return () => {
      active = false;
      void subscription.then((stop) => stop()).catch(() => {});
    };
  }, [nativeWindow]);
  const run = async (action: 'minimize' | 'toggleMaximize' | 'close') => {
    if (!nativeWindow) return;
    setError('');
    try {
      await nativeWindow[action]();
      if (action === 'toggleMaximize') setMaximized(await nativeWindow.isMaximized());
    } catch (reason) {
      setError(describeCommandError(reason));
    }
  };
  return { available: !!nativeWindow, maximized, error, run };
}
