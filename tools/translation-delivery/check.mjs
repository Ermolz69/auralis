import fs from 'node:fs';
import path from 'node:path';
import { assertProductionFrontend, assertProductionResources } from './policy.mjs';

const config = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
assertProductionResources(config);
const frontend = path.resolve('apps/desktop/dist');
const chunks = JSON.parse(fs.readFileSync(path.join(frontend, 'bundle-report.json'), 'utf8'));
const files = [];
function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(target);
    else files.push(path.relative(frontend, target));
  }
}
walk(frontend);
assertProductionFrontend(chunks, files);
console.log(`Translation delivery checks passed: ${files.length} frontend files; no emitted test modules, weights, or test data; bounded native resources.`);
