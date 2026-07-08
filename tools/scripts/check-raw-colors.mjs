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

  let themeTokens = new Set(['transparent', 'current', 'white', 'black']);
  try {
    const themePath = path.join(appsDir, 'desktop', 'src', 'app', 'styles', 'theme.css');
    const themeContent = await fs.readFile(themePath, 'utf-8');
    const tokenMatches = [...themeContent.matchAll(/--color-([a-zA-Z0-9-]+):/g)];
    for (const match of tokenMatches) {
      themeTokens.add(match[1]);
    }
  } catch (err) {
    console.warn('Warning: Could not read theme.css to extract valid tokens.');
  }

  const HEX_REGEX = /#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/g;
  const CLASS_REGEX = /\b(bg|text|border|ring(?:-offset)?|fill|stroke|outline)-([a-zA-Z0-9-]+)(?:\/[0-9]+)?\b/g;
  
  const IGNORE_SUFFIXES = new Set([
    'left', 'center', 'right', 'justify', 'start', 'end',
    'xs', 'sm', 'base', 'lg', 'xl', '2xl', '3xl', '4xl', '5xl', '6xl', '7xl', '8xl', '9xl',
    'none', 'solid', 'dashed', 'dotted', 'double', 'hidden',
    '0', '1', '2', '4', '8', 'inset',
    't', 'r', 'b', 'l', 'x', 'y',
    't-0', 'r-0', 'b-0', 'l-0', 'x-0', 'y-0',
    'clip-text', 'gradient-to-r', 'gradient-to-l', 'gradient-to-t', 'gradient-to-b'
  ]);

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
            
            const hexMatches = [...line.matchAll(HEX_REGEX)];
            if (hexMatches.length > 0) {
              for (const match of hexMatches) {
                const color = match[0];
                let relativePath = path.relative(rootDir, file).replace(/\\/g, '/');
                console.error(`❌ ERROR: Raw hex color '${color}' found in ${relativePath}:${i + 1}`);
                hasErrors = true;
              }
            }

            const classMatches = [...line.matchAll(CLASS_REGEX)];
            if (classMatches.length > 0) {
              for (const match of classMatches) {
                const prefix = match[1];
                const token = match[2];
                if (IGNORE_SUFFIXES.has(token)) continue;
                if (!themeTokens.has(token)) {
                  let relativePath = path.relative(rootDir, file).replace(/\\/g, '/');
                  console.error(`❌ ERROR: Undefined design token '${prefix}-${token}' used in ${relativePath}:${i + 1}`);
                  hasErrors = true;
                }
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
    console.error('\n🚫 Check failed: Invalid color/token usage found.');
    console.error('Please use only valid design tokens declared in theme.css.');
    process.exit(1);
  } else {
    console.log('✅ Success: No raw hex colors or undefined tokens found in restricted directories.');
  }
}

checkColors();
