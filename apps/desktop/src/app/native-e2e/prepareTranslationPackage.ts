import { getTranslationPackageStatus } from '@/entities/translation';

const PACKAGE_TIMEOUT_MS = 900_000;
const POLL_INTERVAL_MS = 200;
const PACKAGE_LABEL = 'Модель перевода';

export async function prepareTranslationPackage(): Promise<boolean> {
  const current = await getTranslationPackageStatus();
  if (!current.supported)
    throw new Error('Pinned CPU package is unsupported on the native E2E host');
  if (current.selected) {
    await waitForSelectedUi();
    return false;
  }
  if (!current.installed) {
    (await waitForButton('Скачать и установить')).click();
  }
  (await waitForButton('Выбрать модель')).click();
  await waitForSelectedUi();
  const selected = await getTranslationPackageStatus();
  if (!selected.installed || !selected.selected) {
    throw new Error('Native package controls did not persist the selection');
  }
  return true;
}

async function waitForButton(label: string): Promise<HTMLButtonElement> {
  const deadline = Date.now() + PACKAGE_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const section = document.querySelector(`section[aria-label="${PACKAGE_LABEL}"]`);
    const alert = section?.querySelector('[role="alert"]');
    if (alert) throw new Error(`Native package control failed: ${alert.textContent?.trim()}`);
    const button = [...(section?.querySelectorAll('button') ?? [])].find(
      (item) => item.textContent?.trim() === label && !item.disabled,
    );
    if (button) return button;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error(`Timed out waiting for native package button: ${label}`);
}

async function waitForSelectedUi(): Promise<void> {
  const deadline = Date.now() + PACKAGE_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const section = document.querySelector(`section[aria-label="${PACKAGE_LABEL}"]`);
    const alert = section?.querySelector('[role="alert"]');
    if (alert) throw new Error(`Native package control failed: ${alert.textContent?.trim()}`);
    if (section?.textContent?.includes('Локальная модель выбрана.')) return;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Timed out waiting for selected package in the native UI');
}

function delay(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
