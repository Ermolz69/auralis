import { useLayoutEffect, useRef } from 'react';
import type { Project } from '@/entities/project';

export type ProjectListFocusTarget = {
  deletedIndex: number;
  deletedProjectId: string;
  reason: 'success' | 'cancel' | 'error';
};

export function useProjectListFocus(projects: Project[]) {
  const pendingFocusTargetRef = useRef<ProjectListFocusTarget | null>(null);
  const deleteButtonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const openButtonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const headingRef = useRef<HTMLHeadingElement>(null);

  useLayoutEffect(() => {
    const target = pendingFocusTargetRef.current;
    if (!target) return;

    if (target.reason === 'cancel' || target.reason === 'error') {
      const btn = deleteButtonRefs.current.get(target.deletedProjectId);
      btn?.focus();
    } else if (target.reason === 'success') {
      if (projects.length === 0) {
        headingRef.current?.focus();
      } else {
        const nextIndex = Math.min(target.deletedIndex, projects.length - 1);
        const nextProjectId = projects[nextIndex]?.id;
        if (nextProjectId) {
          const btn = openButtonRefs.current.get(nextProjectId);
          btn?.focus();
        }
      }
    }
    pendingFocusTargetRef.current = null;
  });

  return { pendingFocusTargetRef, deleteButtonRefs, openButtonRefs, headingRef };
}
