import { createContext } from 'react';
import type { JobStoreState } from './types';

export const JobContext = createContext<JobStoreState | null>(null);
