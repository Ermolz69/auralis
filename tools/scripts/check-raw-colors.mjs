import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');

// We only check these specific directories inside src
const FORBIDDEN_DIRS = [
  'pages',
  'features',
  'widgets',
  'entities',
  'shared/ui'
];

// Matches hex color starting with #, followed by 3, 4, 6, or 8 hex digits
const HEX_REGEX = /#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/g;

async function walkDir(dir) {
  let results = [];
  try {
    const list = await fs.readdir(dir, { withFileTypes: true });
    for (const file of list) {
      const fullPath = path.join(dir, file.name);
      if (file.isDirectory()) {
        results = results.concat(await walkDir(fullPath));
      } else {
        results.push(fullPath);
      }
    }
  } catch (err) {
    if (err.code !== 'ENOENT') {
      console.error(`Error reading directory ${dir}:`, err);
    }
  }
  return results;
}

async function checkColors() {
  const appsDir = path.join(rootDir, 'apps');
  let hasErrors = false;
  let apps = [];

  try {
    apps = await fs.readdir(appsDir);
  } catch (err) {
    if (err.code === 'ENOENT') {
      console.log('No apps directory found. Skipping checks.');
      process.exit(0);
    }
    console.error('Error reading apps directory:', err);
    process.exit(1);
  }

  for (const app of apps) {
    const srcDir = path.join(appsDir, app, 'src');
    
    try {
      const stats = await fs.stat(srcDir);
      if (!stats.isDirectory()) continue;
    } catch (e) {
      continue;
    }

    for (const forbidden of FORBIDDEN_DIRS) {
      const dirToCheck = path.join(srcDir, ...forbidden.split('/'));
      const files = await walkDir(dirToCheck);

      for (const file of files) {
        const ext = path.extname(file).toLowerCase();
        if (!['.ts', '.tsx', '.css', '.scss'].includes(ext)) continue;

        // Skip exceptions
        const lowerFile = file.toLowerCase();
        if (
          lowerFile.includes('theme') || 
          lowerFile.includes('tokens.stories') || 
          lowerFile.includes('designtokens.stories')
        ) {
          continue;
        }

        try {
          const content = await fs.readFile(file, 'utf-8');
          const lines = content.split('\n');
          
          for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const matches = [...line.matchAll(HEX_REGEX)];
            
            if (matches.length > 0) {
              for (const match of matches) {
                const color = match[0];
                let relativePath = path.relative(rootDir, file);
                relativePath = relativePath.replace(/\\/g, '/'); // Normalize slashes for consistent output
                console.error(`❌ ERROR: Raw hex color '${color}' found in ${relativePath}:${i + 1}`);
                hasErrors = true;
              }
            }
          }
        } catch (err) {
          console.error(`Error reading file ${file}:`, err);
        }
      }
    }
  }

  if (hasErrors) {
    console.error('\n🚫 Check failed: Raw hex colors are not allowed in components and pages.');
    console.error('Please use design tokens from the theme instead (e.g. var(--color-primary), bg-primary).');
    process.exit(1);
  } else {
    console.log('✅ Success: No raw hex colors found in restricted directories.');
  }
}

checkColors();
