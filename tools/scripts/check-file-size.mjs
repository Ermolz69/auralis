import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');

const RULES = [
  { dir: 'apps/desktop/src/pages', maxLines: 120, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/widgets', maxLines: 250, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/features', maxLines: 250, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/entities', maxLines: 300, pattern: /\.(ts|tsx)$/ },
  { dir: 'apps/desktop/src/shared/ui', maxLines: 200, pattern: /\.(ts|tsx)$/ },
  { dir: 'crates/application/src', maxLines: 300, pattern: /\.rs$/ },
];

function isExcluded(filePath) {
  if (filePath.includes('node_modules') || filePath.includes('dist') || filePath.includes('target')) return true;
  if (filePath.endsWith('pnpm-lock.yaml') || filePath.endsWith('Cargo.lock')) return true;
  if (filePath.endsWith('.generated.ts') || filePath.endsWith('.d.ts')) return true;
  return false;
}

function walkSync(dir, filelist = []) {
  if (!fs.existsSync(dir)) return filelist;
  const files = fs.readdirSync(dir);
  for (const file of files) {
    const filepath = path.join(dir, file);
    if (fs.statSync(filepath).isDirectory()) {
      filelist = walkSync(filepath, filelist);
    } else {
      filelist.push(filepath);
    }
  }
  return filelist;
}

let hasErrors = false;

for (const rule of RULES) {
  const fullDirPath = path.join(rootDir, rule.dir);
  const files = walkSync(fullDirPath);

  for (const file of files) {
    if (isExcluded(file)) continue;
    if (!rule.pattern.test(file)) continue;

    const content = fs.readFileSync(file, 'utf-8');
    const lines = content.split('\n').length;
    
    if (lines > rule.maxLines) {
      console.error(`ERROR: File too large! [${lines}/${rule.maxLines} lines] ${path.relative(rootDir, file)}`);
      hasErrors = true;
    }
  }
}

if (hasErrors) {
  process.exit(1);
} else {
  console.log("SUCCESS: All file sizes are within architectural limits.");
}
