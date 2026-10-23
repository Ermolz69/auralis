import { describeCommandError } from '@/shared/api/contracts/error';
import { useState } from 'react';
import { useLibrary } from '@/entities/library';
import { createProject } from '@/entities/project';
import { createText, importYoutube, type Destination } from '@/entities/library';
import { button, field } from './controls';
import { Button } from '@/shared/ui/button';
type Props = {
  kind: 'text' | 'project' | 'folder' | 'youtube';
  destination: Destination;
  close: () => void;
  inWorkspace?: boolean;
};
export function LibraryCreateDialog({ kind, destination, close, inWorkspace }: Props) {
  const { mutate, refresh, openAsset, openTool } = useLibrary();
  const [name, setName] = useState('');
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const titles = {
    text: 'Новый текст',
    project: 'Новый проект',
    folder: 'Новая папка',
    youtube: 'Добавить из YouTube',
  };
  const submit = async () => {
    setBusy(true);
    setError('');
    try {
      if (kind === 'text') {
        const asset = await createText(name.trim(), text, destination, mutate);
        await refresh();
        if (inWorkspace) await openTool('text', asset.id);
        else await openAsset(asset, undefined, destination.projectId);
      } else if (kind === 'project') {
        await createProject(name.trim());
        await refresh();
      } else if (kind === 'youtube') await importYoutube(name.trim(), destination, mutate, refresh);
      else if (destination.projectId)
        await mutate((state) => {
          state.folders.push({
            id: crypto.randomUUID(),
            projectId: destination.projectId!,
            parentId: destination.folderId,
            name: name.trim(),
          });
        });
      close();
    } catch (reason) {
      setError(describeCommandError(reason));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/90 p-4"
      onKeyDown={(e) => {
        if (e.key === 'Escape' && !busy) close();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="library-create-title"
        className="max-h-[90vh] w-full max-w-xl overflow-auto border border-border bg-bg p-5"
      >
        <h2 id="library-create-title" className="mb-4 text-xl font-semibold">
          {titles[kind]}
        </h2>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
          className="space-y-4"
        >
          <label className="block">
            {kind === 'youtube' ? 'Ссылка YouTube' : 'Название'}
            <input
              autoFocus
              required
              maxLength={200}
              type={kind === 'youtube' ? 'url' : 'text'}
              className={field}
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={busy}
            />
          </label>
          {kind === 'text' && (
            <label className="block">
              Текст
              <textarea
                className={`${field} min-h-48`}
                value={text}
                onChange={(e) => setText(e.target.value)}
                disabled={busy}
                maxLength={250000}
              />
            </label>
          )}
          {error && <p role="alert">{error}</p>}
          <div className="flex justify-end gap-2">
            <button type="button" className={button} onClick={close} disabled={busy}>
              Отмена
            </button>
            <Button type="submit" disabled={busy || !name.trim()}>
              {busy ? 'Сохраняем…' : 'Создать'}
            </Button>
          </div>
        </form>
      </section>
    </div>
  );
}
