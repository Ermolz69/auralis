import { useState } from 'react';
import type { ReactNode } from 'react';
import { NavigationContext } from './context';
import type { View } from './types';

export function NavigationProvider({ children }: { children: ReactNode }) {
  const [currentView, setCurrentView] = useState<View>('home');

  return (
    <NavigationContext.Provider value={{ currentView, setCurrentView }}>
      {children}
    </NavigationContext.Provider>
  );
}
