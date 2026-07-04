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
  { dir: 'apps/desktop/src/shared/lib', maxLines: 250, pattern: /\.(ts|tsx)$/ },
  { dir: 'crates/application/src', maxLines: 300, pattern: /\.rs$/ },
];

function getRuleForFile(filePath) {
  // Check explicit rules
  for (const rule of RULES) {
    const fullDirPath = path.join(rootDir, path.normalize(rule.dir));
    if (filePath.startsWith(fullDirPath) && rule.pattern.test(filePath)) {
      return rule;
    }
  }
  // Check dynamic rules for Rust adapters
  if (filePath.includes(`${path.sep}adapters-`) && filePath.endsWith('.rs')) {
    return { maxLines: 400 };
  }
  return null;
}

function isExcluded(filePath) {
  if (filePath.includes('node_modules') || filePath.includes('dist') || filePath.includes('target')) return true;
  if (filePath.endsWith('pnpm-lock.yaml') || filePath.endsWith('Cargo.lock')) return true;
  if (filePath.endsWith('.generated.ts') || filePath.endsWith('.d.ts')) return true;
  if (filePath.includes('__generated__') || filePath.includes('api-types')) return true;
  if (filePath.endsWith('.snap') || filePath.endsWith('.svg')) return true;
  
  // Exclude large static data configs
  const basename = path.basename(filePath).toLowerCase();
  if (basename.includes('mock') || basename.endsWith('data.ts') || basename.endsWith('constants.ts')) {
    return true;
  }
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

// Gather all files from relevant roots
const searchRoots = [
  path.join(rootDir, 'apps/desktop/src'),
  path.join(rootDir, 'crates')
];

for (const searchRoot of searchRoots) {
  const files = walkSync(searchRoot);
  for (const file of files) {
    if (isExcluded(file)) continue;

    const rule = getRuleForFile(file);
    if (!rule) continue;

    const content = fs.readFileSync(file, 'utf-8');
    const lines = content.split('\n').length;
    
    if (lines > rule.maxLines) {
      console.error(`\nERROR: File too large! [${lines}/${rule.maxLines} lines] -> ${path.relative(rootDir, file)}`);
      console.error(`       Policy to fix this:`);
      console.error(`       1. Pure functions -> move to lib`);
      console.error(`       2. Complex UI -> split into smaller components`);
      console.error(`       3. State logic -> move to model`);
      console.error(`       4. DTO mapping -> move to api/lib`);
      console.error(`       5. Constants/Static Data -> move to config`);
      hasErrors = true;
    }
  }
}

if (hasErrors) {
  process.exit(1);
} else {
  console.log("SUCCESS: All file sizes are within architectural limits.");
}
