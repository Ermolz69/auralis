import type { Project } from './types';

export type ProjectSelection = { status: 'closed' } | { status: 'open'; project: Project };
