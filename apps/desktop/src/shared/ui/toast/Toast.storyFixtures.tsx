import { useState } from 'react';
import { Button } from '../button';
import { toast } from './toast';
import { FOCUS_TOAST_DURATION } from './Toast.lifecycle.storyFixtures';

export function ActionFeedback({ finite = false }: { finite?: boolean }) {
  const [activations, setActivations] = useState(0);
  return (
    <div className="flex flex-col gap-3">
      <Button
        onClick={() =>
          toast.warning('Action available', {
            duration: finite ? FOCUS_TOAST_DURATION : 0,
            action: {
              label: 'Retry action',
              onClick: () => setActivations((value) => value + 1),
            },
          })
        }
      >
        Show action feedback
      </Button>
      <Button variant="secondary">Return to page</Button>
      <p role="status" aria-label="Action activations">
        {activations}
      </p>
    </div>
  );
}

export function SourceActionFeedback() {
  const [opened, setOpened] = useState(0);
  return (
    <div>
      <Button
        onClick={() =>
          toast.warning('Источник не выбран', {
            description: 'Подключите локальное видео или ссылку YouTube, чтобы продолжить.',
            duration: 0,
            action: { label: 'Открыть источник', onClick: () => setOpened((count) => count + 1) },
          })
        }
      >
        Show Action Toast
      </Button>
      <p role="status" aria-label="Source actions">
        {opened}
      </p>
    </div>
  );
}
