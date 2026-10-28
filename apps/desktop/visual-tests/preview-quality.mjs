import { readFileSync } from 'node:fs';
import { expect } from 'playwright/test';

const { themes } = JSON.parse(
  readFileSync(new URL('../src/shared/theme/config/tesseraTokens.json', import.meta.url), 'utf8'),
);

function normalizeHex(color) {
  return /^#[\da-f]{3}$/.test(color)
    ? `#${[...color.slice(1)].map((digit) => digit.repeat(2)).join('')}`
    : color;
}

function rgb(hex) {
  return `rgb(${hex
    .slice(1)
    .match(/.{2}/g)
    .map((pair) => Number.parseInt(pair, 16))
    .join(', ')})`;
}

export async function assertPreviewQuality(page, theme, runtimeErrors) {
  if (!Object.hasOwn(themes, theme)) throw new Error(`Unknown visual theme: ${theme}`);
  await expect(page.locator('#storybook-root')).not.toBeEmpty();
  await expect(page.locator('html')).toHaveAttribute('data-color-theme', theme);
  await expect(page.locator('#storybook-root > [data-storybook-color-theme]')).toHaveAttribute(
    'data-storybook-color-theme',
    theme,
  );
  await expect(page.locator('.sb-errordisplay:visible, .sb-nopreview:visible')).toHaveCount(0);
  const appearance = await page.evaluate(() => {
    const style = getComputedStyle(document.documentElement);
    const body = getComputedStyle(document.body);
    return {
      scheme: style.colorScheme,
      background: style.getPropertyValue('--color-bg').trim().toLowerCase(),
      text: style.getPropertyValue('--color-text').trim().toLowerCase(),
      bodyBackground: body.backgroundColor,
      bodyText: body.color,
      width: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth),
      viewport: window.innerWidth,
    };
  });
  expect(appearance.scheme).toBe(theme);
  expect(normalizeHex(appearance.background)).toBe(themes[theme].canvas.toLowerCase());
  expect(normalizeHex(appearance.text)).toBe(themes[theme].text.toLowerCase());
  expect(appearance.bodyBackground).toBe(rgb(themes[theme].canvas));
  expect(appearance.bodyText).toBe(rgb(themes[theme].text));
  expect(appearance.width).toBeLessThanOrEqual(appearance.viewport + 1);
  expect(runtimeErrors).toEqual([]);
}

export async function assertReachableControl(control) {
  await expect(control).toBeVisible();
  await expect(control).toBeEnabled();
  await control.scrollIntoViewIfNeeded();
  const reachability = await control.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    const insetX = Math.min(2, rect.width / 4);
    const insetY = Math.min(2, rect.height / 4);
    const points = [
      [rect.left + rect.width / 2, rect.top + rect.height / 2],
      [rect.left + insetX, rect.top + insetY],
      [rect.right - insetX, rect.top + insetY],
      [rect.left + insetX, rect.bottom - insetY],
      [rect.right - insetX, rect.bottom - insetY],
    ];
    return {
      insideViewport:
        rect.left >= 0 && rect.right <= innerWidth && rect.top >= 0 && rect.bottom <= innerHeight,
      receivesPointer: points.every(([x, y]) => {
        const hit = document.elementFromPoint(x, y);
        return Boolean(hit && (hit === element || element.contains(hit)));
      }),
    };
  });
  expect(reachability).toEqual({ insideViewport: true, receivesPointer: true });
}
