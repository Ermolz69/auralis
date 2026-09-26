const TIMEOUT_MS = 60_000;
const POLL_INTERVAL_MS = 25;

export class TranslationHistoryControls {
  readonly container: HTMLDivElement;

  constructor(container: HTMLDivElement) {
    this.container = container;
  }

  async wait(predicate: () => boolean | Promise<boolean>): Promise<void> {
    const deadline = Date.now() + TIMEOUT_MS;
    while (Date.now() < deadline) {
      if (await predicate()) return;
      await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    }
    throw new Error(`History controls timed out: ${this.container.textContent?.slice(-1200)}`);
  }

  button(label: string, scope: ParentNode = this.container): HTMLButtonElement | undefined {
    return Array.from(scope.querySelectorAll('button')).find(
      (item) => item.textContent?.trim() === label && !item.disabled,
    );
  }

  hasParagraph(text: string): boolean {
    return Array.from(this.container.querySelectorAll('p')).some(
      (item) => item.textContent === text,
    );
  }

  async preview(resultId: string, expectedText: string): Promise<void> {
    await this.wait(() => {
      const selector = this.container.querySelector<HTMLSelectElement>(
        '#translation-history-selection',
      );
      return Boolean(
        selector &&
        !selector.disabled &&
        Array.from(selector.options).some((item) => item.value === resultId),
      );
    });
    const selector = this.container.querySelector<HTMLSelectElement>(
      '#translation-history-selection',
    )!;
    selector.value = resultId;
    selector.dispatchEvent(new Event('change', { bubbles: true }));
    await this.wait(() => this.hasParagraph(expectedText));
  }

  async open(sourceText: string): Promise<HTMLDivElement> {
    await this.wait(() => Boolean(this.segment(sourceText)));
    const segment = this.segment(sourceText)!;
    await this.wait(() => Boolean(this.button('Исправить перевод', segment)));
    this.button('Исправить перевод', segment)!.click();
    await this.wait(() => Boolean(this.button('Сохранить новую версию', segment)));
    return segment;
  }

  async edit(sourceText: string, replacement: string): Promise<void> {
    const segment = await this.open(sourceText);
    const input = segment.querySelector<HTMLInputElement>('input[type="text"]');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    if (!input || !setter) throw new Error('Native historical editor input is unavailable');
    setter.call(input, replacement);
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await this.wait(
      () => Boolean(this.button('Сохранить новую версию', segment)) && input.value === replacement,
    );
    this.button('Сохранить новую версию', segment)!.click();
  }

  private segment(sourceText: string): HTMLDivElement | null {
    const paragraph = Array.from(this.container.querySelectorAll('p')).find(
      (item) => item.textContent === sourceText,
    );
    return paragraph?.closest<HTMLDivElement>('div.grid') ?? null;
  }
}
