import { createRoot } from 'react-dom/client';
import { TranslationHistoryWorkspace } from './TranslationHistoryWorkspace';

export function mountTranslationReview(projectId: string) {
  const container = document.createElement('div');
  document.body.append(container);
  const root = createRoot(container);
  root.render(<TranslationHistoryWorkspace projectId={projectId} />);
  return {
    container,
    dispose: () => {
      root.unmount();
      container.remove();
    },
  };
}
