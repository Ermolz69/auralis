import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { DEFAULT_COLOR_THEME, isColorTheme, type ColorTheme } from '../config/colorThemes';
import { applyColorTheme } from './colorThemeStorage';
import type { ThemePersistence } from './persistence';
import { ThemeContext } from './ThemeContext';

export function ThemeProvider({
  children,
  persistence,
}: {
  children: ReactNode;
  persistence?: ThemePersistence;
}) {
  const [colorTheme, setColorThemeState] = useState<ColorTheme>(DEFAULT_COLOR_THEME);
  const [persistenceStatus, setPersistenceStatus] = useState<
    'idle' | 'pending' | 'saved' | 'error'
  >('pending');
  const sequence = useRef(0);
  const mounted = useRef(false);
  const localPending = useRef(false);
  useEffect(() => {
    mounted.current = true;
    if (!persistence) {
      setPersistenceStatus('idle');
      return () => {
        mounted.current = false;
      };
    }
    const request = ++sequence.current;
    const unsubscribe = persistence.subscribe((stored) => {
      if (!localPending.current && isColorTheme(stored.value)) setColorThemeState(stored.value);
    });
    void persistence
      .load()
      .then((stored) => {
        if (!mounted.current || request !== sequence.current) return;
        if (stored && isColorTheme(stored.value)) setColorThemeState(stored.value);
        setPersistenceStatus(stored && !isColorTheme(stored.value) ? 'error' : 'idle');
      })
      .catch(() => {
        if (mounted.current && request === sequence.current) setPersistenceStatus('error');
      });
    return () => {
      mounted.current = false;
      sequence.current += 1;
      unsubscribe();
    };
  }, [persistence]);

  useLayoutEffect(() => {
    applyColorTheme(colorTheme);
  }, [colorTheme]);

  const setColorTheme = useCallback(
    (theme: ColorTheme) => {
      const request = ++sequence.current;
      localPending.current = true;
      setColorThemeState(theme);
      if (!persistence) {
        localPending.current = false;
        setPersistenceStatus('idle');
        return;
      }
      setPersistenceStatus('pending');
      void persistence
        .save(theme)
        .then((stored) => {
          if (mounted.current && request === sequence.current) {
            localPending.current = false;
            if (isColorTheme(stored.value)) setColorThemeState(stored.value);
            setPersistenceStatus('saved');
          }
        })
        .catch(() => {
          if (mounted.current && request === sequence.current) {
            localPending.current = false;
            setPersistenceStatus('error');
          }
        });
    },
    [persistence],
  );

  const value = useMemo(
    () => ({ colorTheme, setColorTheme, persistenceStatus }),
    [colorTheme, setColorTheme, persistenceStatus],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}
