import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');
const srcDir = path.join(rootDir, 'apps/desktop/src');

const PATTERNS = [
  { regex: /bg-\[#[0-9a-fA-F]{3,8}\]/, name: 'Tailwind arbitrary bg color (bg-[#...])' },
  { regex: /text-\[#[0-9a-fA-F]{3,8}\]/, name: 'Tailwind arbitrary text color (text-[#...])' },
  { regex: /border-\[#[0-9a-fA-F]{3,8}\]/, name: 'Tailwind arbitrary border color (border-[#...])' },
  { regex: /(color|backgroundColor|borderColor)\s*:\s*['"`]#[0-9a-fA-F]{3,8}['"`]/, name: 'Inline style with raw color (style={{ color: ... }})' },
  { regex: /['"`]#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b['"`]/, name: 'Raw hex color string' }
];

function isExcluded(filePath) {
  // Allow theme and root css files where variables are officially defined
  if (filePath.includes('theme') || filePath.includes('index.css') || filePath.includes('global.css')) return true;
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
const files = walkSync(srcDir);

for (const file of files) {
  if (!file.match(/\.(ts|tsx|css|scss)$/)) continue;
  if (isExcluded(file)) continue;

  const content = fs.readFileSync(file, 'utf-8');
  const lines = content.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    for (const pattern of PATTERNS) {
      if (pattern.regex.test(line)) {
        console.error(`\nERROR: Raw color found in ${path.relative(rootDir, file)}:${i + 1}`);
        console.error(`       Violating pattern: ${pattern.name}`);
        console.error(`       Code: ${line.trim()}`);
        console.error(`       Fix: Please use design system theme tokens instead of raw colors.`);
        console.error(`            Example: use Tailwind classes (e.g., bg-surface) or CSS variables (var(--color-bg)).`);
        hasErrors = true;
      }
    }
  }
}

if (hasErrors) {
  process.exit(1);
} else {
  console.log("SUCCESS: No raw colors found. Design system tokens are properly enforced.");
}
