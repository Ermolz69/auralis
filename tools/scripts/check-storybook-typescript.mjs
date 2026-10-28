import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

export function inside(root, file) {
  const relative = path.relative(root, file);
  return relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

export function loadStorybookTypeScript(rootDir) {
  const root = fs.realpathSync(rootDir);
  const require = createRequire(path.join(root, 'apps/desktop/package.json'));
  const resolved = require.resolve('typescript');
  const real = fs.realpathSync(resolved);
  if (!inside(root, resolved) || !inside(root, real)) {
    throw new Error('Storybook checker requires TypeScript installed inside the owned repository');
  }
  return { ts: require(real), path: real, root };
}
