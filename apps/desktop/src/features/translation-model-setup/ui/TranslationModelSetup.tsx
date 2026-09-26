import { useEffect, useState } from 'react';
import {
  getTranslationPackageStatus,
  installTranslationPackage,
  selectTranslationPackage,
} from '@/entities/translation';
import { toCommandError } from '@/shared/api/contracts';
import type { TranslationPackageStatus } from '@/shared/api/contracts/translation';
import { Button } from '@/shared/ui/button';

function failureMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : toCommandError(reason).message;
}

export function TranslationModelSetup() {
  const [status, setStatus] = useState<TranslationPackageStatus | null>(null);
  const [busy, setBusy] = useState<'install' | 'select' | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void getTranslationPackageStatus()
      .then((current) => {
        if (active) setStatus(current);
      })
      .catch((reason) => {
        if (active) setError(failureMessage(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  const install = async () => {
    if (busy) return;
    setBusy('install');
    setError(null);
    try {
      setStatus(await installTranslationPackage());
    } catch (reason) {
      setError(failureMessage(reason));
    } finally {
      setBusy(null);
    }
  };

  const select = async () => {
    if (busy || !status) return;
    setBusy('select');
    setError(null);
    try {
      setStatus(await selectTranslationPackage(status.releaseId, status.backend));
    } catch (reason) {
      setError(failureMessage(reason));
    } finally {
      setBusy(null);
    }
  };

  return (
    <section
      className="rounded-md border border-border bg-surface p-4"
      aria-label="Модель перевода"
    >
      <h2 className="text-sm font-semibold text-text">Модель перевода</h2>
      <p className="mt-1 text-xs text-muted">
        {status && !status.supported
          ? 'Этот экспериментальный пакет поддерживает только Windows x64.'
          : status?.selected
            ? 'Локальная модель выбрана. Перед запуском её файлы будут проверены.'
            : status?.installed
              ? 'Пакет установлен. Выберите его для новых переводов.'
              : 'Для локального перевода установите проверенный пакет модели и llama.cpp.'}
      </p>
      {status?.supported && !status.installed && (
        <p className="mt-1 text-xs text-muted">
          Загрузка около 1,2 ГБ. Файлы проверяются перед установкой; прерванную загрузку можно
          продолжить.
        </p>
      )}
      {error && (
        <p role="alert" className="mt-2 text-xs text-danger">
          {error}
        </p>
      )}
      {status?.supported && !status.installed && (
        <Button
          className="mt-3"
          size="sm"
          onClick={() => void install()}
          loading={busy === 'install'}
        >
          {busy === 'install' ? 'Загрузка и проверка…' : 'Скачать и установить'}
        </Button>
      )}
      {status?.supported && status.installed && !status.selected && (
        <Button
          className="mt-3"
          size="sm"
          onClick={() => void select()}
          loading={busy === 'select'}
          disabled={busy === 'install'}
        >
          Выбрать модель
        </Button>
      )}
      {status?.supported && status.installed && (
        <Button
          className="mt-3 ml-2"
          size="sm"
          variant="secondary"
          onClick={() => void install()}
          loading={busy === 'install'}
          disabled={busy === 'select'}
        >
          {busy === 'install' ? 'Проверка и восстановление…' : 'Проверить и восстановить'}
        </Button>
      )}
    </section>
  );
}
