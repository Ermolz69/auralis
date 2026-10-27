import { useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { describeCommandError } from '@/shared/api/contracts/error';

export function useWindowChrome() {
  const [initial] = useState(() => {
    try {
      return { window: isTauri() ? getCurrentWindow() : null, error: '' };
    } catch (reason) {
      return { window: null, error: describeCommandError(reason) };
    }
  });
  const nativeWindow = initial.window;
  const [maximized, setMaximized] = useState(false);
  const [error, setError] = useState(initial.error);
  const pending = useRef(false);
  const mounted = useRef(false);
  const sequence = useRef(0);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      sequence.current++;
    };
  }, []);
  useEffect(() => {
    if (!nativeWindow) return;
    let active = true;
    const sync = async () => {
      const request = ++sequence.current;
      try {
        const value = await nativeWindow.isMaximized();
        if (active && sequence.current === request) setMaximized(value);
      } catch (reason) {
        if (active && sequence.current === request) setError(describeCommandError(reason));
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
    if (!nativeWindow || pending.current) return;
    pending.current = true;
    setError('');
    try {
      await nativeWindow[action]();
      if (action === 'toggleMaximize') {
        const request = ++sequence.current;
        const value = await nativeWindow.isMaximized();
        if (mounted.current && request === sequence.current) setMaximized(value);
      }
    } catch (reason) {
      if (mounted.current) setError(describeCommandError(reason));
    } finally {
      pending.current = false;
    }
  };
  return { available: !!nativeWindow, maximized, error, run };
}
