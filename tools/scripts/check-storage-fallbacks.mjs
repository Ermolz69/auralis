import fs from 'fs';
import path from 'path';

const dirs = [
  'crates/application/src',
  'crates/jobs/src',
  'crates/adapters-storage/src',
  'crates/adapters-tauri/src',
  'crates/adapters-ytdlp/src',
  'crates/adapters-ffmpeg/src',
  'crates/adapters-model/src',
  'crates/ports/src',
  'src-tauri/src'
];

let hasError = false;

function scanDir(dir) {
  if (!fs.existsSync(dir)) return;
  const files = fs.readdirSync(dir);
  for (const file of files) {
    const fullPath = path.join(dir, file);
    const stat = fs.statSync(fullPath);
    if (stat.isDirectory()) {
      scanDir(fullPath);
    } else if (fullPath.endsWith('.rs')) {
      checkFile(fullPath);
    }
  }
}

function checkFile(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    
    // Ignore comments
    if (line.trim().startsWith('//')) {
      continue;
    }
    
    // Check for explicit allow
    if (line.includes('allow-fallback')) {
      continue;
    }

    // Naive string check (not perfect but covers our use case without a full AST parser)
    if (line.includes('.unwrap_or_default()') && !line.includes('"')) {
      console.error(`[ERROR] Storage fallback found: unwrap_or_default() at ${filePath}:${i + 1}`);
      hasError = true;
    }
    if (line.match(/let\s+_\s*=\s*.*\.(commit|rollback)\(\)/)) {
      console.error(`[ERROR] Ignored persistence result: let _ = ... at ${filePath}:${i + 1}`);
      hasError = true;
    }

    // Check for artifacts_json leakage outside of allowed files
    if (line.includes('artifacts_json')) {
      const isAllowedFile = 
        filePath.replace(/\\/g, '/').includes('/sqlite/migrations_runtime/backfill_artifacts.rs') ||
        filePath.replace(/\\/g, '/').includes('/sqlite/migrations_runtime/tests.rs') ||
        filePath.replace(/\\/g, '/').includes('/sqlite/preflight/tests.rs') ||
        filePath.replace(/\\/g, '/').includes('/sqlite/preflight/inspector.rs') ||
        filePath.replace(/\\/g, '/').includes('/sqlite/preflight/state_machine.rs');
        
      if (!isAllowedFile) {
        console.error(`[ERROR] Legacy artifacts_json used outside of migration runtime at ${filePath}:${i + 1}`);
        hasError = true;
      }
    }
  }
}

dirs.forEach(scanDir);

if (hasError) {
  process.exit(1);
}
