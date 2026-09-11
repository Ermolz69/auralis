import { useEffect, useReducer } from 'react';
import { subscribeProjectPreferences } from './preferencesStorage';
import {
  getPinLoadStatus,
  getPinPersistence,
  getUnsavedPinIds,
  loadProjectPins,
  setProjectPinned,
} from './pinPersistence';

export function usePinPersistence(projectId?: string) {
  const [, refresh] = useReducer((value: number) => value + 1, 0);
  useEffect(() => {
    const unsubscribe = subscribeProjectPreferences(() => refresh());
    void loadProjectPins().catch(() => undefined);
    return unsubscribe;
  }, []);
  return {
    ...(projectId ? getPinPersistence(projectId) : { status: 'saved', desired: undefined }),
    loadStatus: getPinLoadStatus(),
    unsavedIds: getUnsavedPinIds(),
    retryLoad: () => {
      void loadProjectPins().catch(() => undefined);
    },
    retry: (id: string) => {
      const value = getPinPersistence(id).desired;
      if (value !== undefined) void setProjectPinned(id, value);
    },
  };
}
