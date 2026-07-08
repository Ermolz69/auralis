import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');
const srcDir = path.join(rootDir, 'apps/desktop/src');

const LAYERS_WITH_SLICES = ['app', 'pages', 'widgets', 'features', 'entities'];

function getFsdInfo(absolutePath) {
  if (!absolutePath.startsWith(srcDir)) return null;
  const relative = path.relative(srcDir, absolutePath).replace(/\\/g, '/');
  const parts = relative.split('/');
  
  if (parts.length === 0 || parts[0] === '') return null;
  
  const layer = parts[0];
  
  if (!LAYERS_WITH_SLICES.includes(layer)) {
    return { layer, isSlice: false, relativePath: relative };
  }
  
  if (parts.length < 2) return null;
  const slice = parts[1];
  return { layer, slice, relativePath: relative };
}

function validateImport(sourceFile, importString) {
  let targetPath;
  if (importString.startsWith('@/')) {
    targetPath = path.join(srcDir, importString.slice(2));
  } else if (importString.startsWith('.')) {
    targetPath = path.resolve(path.dirname(sourceFile), importString);
  } else {
    return null; // External or node_module
  }

  const sourceInfo = getFsdInfo(sourceFile);
  const targetInfo = getFsdInfo(targetPath);

  if (!targetInfo) return null;
  if (targetInfo.layer === 'shared') return null; // allow deep imports to shared (e.g. shared/ui/button)

  if (targetInfo.isSlice !== false) {
    if (sourceInfo && sourceInfo.layer === targetInfo.layer && sourceInfo.slice === targetInfo.slice) {
      return null; // intra-slice is allowed
    }

    const parts = targetInfo.relativePath.split('/');
    // Public API is either the slice folder itself or slice/index.ts
    // In our parts array: ['entities', 'user'] => length 2
    // Or ['entities', 'user', 'index.ts'] => length 3
    if (parts.length === 2) return null;
    
    // Sometimes imports might not have an extension.
    if (parts.length === 3 && parts[2].startsWith('index')) return null;

    return `Forbidden deep import into slice '${targetInfo.layer}/${targetInfo.slice}'. Target: ${targetInfo.relativePath}`;
  }

  return null;
}

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

// Regex to catch imports and exports
const REGEXES = [
  /import\s+(?:type\s+)?(?:{[^}]*}|.*?)\s+from\s+['"]([^'"]+)['"]/g,
  /export\s+(?:type\s+)?(?:{[^}]*}|\*)\s+from\s+['"]([^'"]+)['"]/g,
  /import\(['"]([^'"]+)['"]\)/g
];

async function checkFsdImports() {
  let hasErrors = false;
  
  // We only run this on apps/desktop/src for now
  const files = await walkDir(srcDir);
  
  for (const file of files) {
    const ext = path.extname(file).toLowerCase();
    if (!['.ts', '.tsx'].includes(ext)) continue;

    try {
      const content = await fs.readFile(file, 'utf-8');
      const lines = content.split('\n');
      
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        
        for (const regex of REGEXES) {
          const matches = [...line.matchAll(regex)];
          for (const match of matches) {
            const importString = match[1];
            const error = validateImport(file, importString);
            
            if (error) {
              const relativePath = path.relative(rootDir, file).replace(/\\/g, '/');
              console.error(`❌ ERROR in ${relativePath}:${i + 1} -> ${error}`);
              console.error(`   Import: '${importString}'`);
              hasErrors = true;
            }
          }
        }
      }
    } catch (err) {
      console.error(`Error processing file ${file}:`, err);
    }
  }

  if (hasErrors) {
    console.error('\n🚫 Check failed: Cross-layer or cross-slice imports must only use public APIs (index.ts).');
    process.exit(1);
  } else {
    console.log('✅ Success: No forbidden deep FSD imports found.');
  }
}

// Simple internal test runner
function runTests() {
  console.log('Running internal script tests...');
  
  const testCases = [
    // 1. Intra-slice relative import -> Allowed
    { source: path.join(srcDir, 'entities/user/ui/UserCard.tsx'), importStr: '../model/store', expectedError: null },
    // 2. Cross-slice relative public API import -> Allowed
    { source: path.join(srcDir, 'features/login/ui/LoginForm.tsx'), importStr: '../../../entities/user', expectedError: null },
    // 3. Cross-slice relative deep import -> Forbidden
    { source: path.join(srcDir, 'features/login/ui/LoginForm.tsx'), importStr: '../../../entities/user/model/store', expectedError: `Forbidden deep import into slice 'entities/user'. Target: entities/user/model/store` },
    // 4. Cross-slice alias deep import -> Forbidden
    { source: path.join(srcDir, 'features/login/ui/LoginForm.tsx'), importStr: '@/entities/user/model/store', expectedError: `Forbidden deep import into slice 'entities/user'. Target: entities/user/model/store` },
    // 5. Shared layer import -> Allowed
    { source: path.join(srcDir, 'features/login/ui/LoginForm.tsx'), importStr: '@/shared/ui/button/Button', expectedError: null },
  ];
  
  let failed = false;
  for (const [index, tc] of testCases.entries()) {
    const error = validateImport(tc.source, tc.importStr);
    if (error !== tc.expectedError) {
      console.error(`Test ${index + 1} failed!`);
      console.error(`  Expected: ${tc.expectedError}`);
      console.error(`  Got:      ${error}`);
      failed = true;
    }
  }
  
  if (failed) {
    console.error('Internal tests failed! Aborting check.');
    process.exit(1);
  }
  console.log('Internal tests passed.');
}

if (process.argv.includes('--test')) {
  runTests();
  process.exit(0);
}

// Run tests before checking project files, to ensure script sanity
runTests();
checkFsdImports();
