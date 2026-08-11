import { useState } from 'react';
import type { ReactNode } from 'react';
import { NavigationContext } from './context';
import type { PipelineStep, View } from './types';

export function NavigationProvider({ children }: { children: ReactNode }) {
  const [currentView, setCurrentView] = useState<View>('home');
  const [pipelineStep, setPipelineStep] = useState<PipelineStep>('source');

  return (
    <NavigationContext.Provider
      value={{ currentView, setCurrentView, pipelineStep, setPipelineStep }}
    >
      {children}
    </NavigationContext.Provider>
  );
}
