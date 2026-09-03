import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

const kib = 1024;
export const bundleBudgets = {
  initialJs: 350 * kib,
  totalJs: 500 * kib,
  largestJs: 250 * kib,
  initialGzip: 110 * kib,
  totalGzip: 160 * kib,
  totalCss: 100 * kib,
};

export function measureBundle(manifest, readAsset) {
  const entries = Object.keys(manifest).filter((key) => manifest[key].isEntry);
  if (entries.length === 0) throw new Error('Bundle manifest contains no entry point');
  const initialKeys = new Set();
  const visit = (key) => {
    if (initialKeys.has(key)) return;
    const chunk = manifest[key];
    if (!chunk) throw new Error(`Missing imported chunk: ${key}`);
    initialKeys.add(key);
    for (const dependency of chunk.imports ?? []) visit(dependency);
  };
  entries.forEach(visit);
  const initialFiles = new Set([...initialKeys].map((key) => manifest[key].file));
  const jsFiles = new Set(
    Object.values(manifest)
      .map((chunk) => chunk.file)
      .filter((file) => file.endsWith('.js')),
  );
  if (jsFiles.size === 0) throw new Error('Bundle contains no JavaScript');
  const cssFiles = new Set(Object.values(manifest).flatMap((chunk) => chunk.css ?? []));
  const result = {
    initialJs: 0,
    totalJs: 0,
    largestJs: 0,
    initialGzip: 0,
    totalGzip: 0,
    totalCss: 0,
  };
  for (const file of jsFiles) {
    const contents = readAsset(file);
    const size = Buffer.byteLength(contents);
    const gzip = gzipSync(contents).length;
    result.totalJs += size;
    result.totalGzip += gzip;
    result.largestJs = Math.max(result.largestJs, size);
    if (initialFiles.has(file)) {
      result.initialJs += size;
      result.initialGzip += gzip;
    }
  }
  for (const file of cssFiles) result.totalCss += Buffer.byteLength(readAsset(file));
  return result;
}

export function budgetViolations(sizes, budgets = bundleBudgets) {
  return Object.entries(budgets).flatMap(([name, limit]) =>
    !Number.isFinite(sizes[name]) || sizes[name] > limit
      ? [`${name}: ${sizes[name]} bytes exceeds ${limit} bytes`]
      : [],
  );
}

function main() {
  const root = path.resolve('apps/desktop/dist');
  const manifest = JSON.parse(readFileSync(path.join(root, '.vite/manifest.json'), 'utf8'));
  const sizes = measureBundle(manifest, (file) => {
    const target = path.resolve(root, file);
    if (!target.startsWith(root + path.sep)) throw new Error('Asset escapes bundle directory');
    return readFileSync(target);
  });
  console.table(
    Object.entries(sizes).map(([metric, bytes]) => ({
      metric,
      KiB: (bytes / kib).toFixed(2),
      limitKiB: bundleBudgets[metric] / kib,
    })),
  );
  const problems = budgetViolations(sizes);
  if (problems.length) throw new Error(problems.join('\n'));
  const report = JSON.parse(readFileSync(path.join(root, 'bundle-report.json'), 'utf8'));
  console.log('Largest rendered modules (before final chunk compression):');
  console.table(
    report
      .flatMap((chunk) => chunk.modules)
      .sort((a, b) => b.renderedBytes - a.renderedBytes)
      .slice(0, 10),
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
