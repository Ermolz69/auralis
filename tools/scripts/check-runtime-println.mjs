import fs from 'fs/promises';
import path from 'path';

const IGNORED_DIRS = ['tests', 'examples', 'benches'];

async function searchDir(dir) {
  let hasError = false;
  const entries = await fs.readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    if (entry.isDirectory()) {
      if (!IGNORED_DIRS.includes(entry.name)) {
        if (await searchDir(path.join(dir, entry.name))) {
            hasError = true;
        }
      }
    } else if (entry.isFile() && entry.name.endsWith('.rs')) {
      if (entry.name.endsWith('_tests.rs') || entry.name === 'tests.rs') {
        continue;
      }
      const fullPath = path.join(dir, entry.name);
      const content = await fs.readFile(fullPath, 'utf-8');
      
      const lines = content.split('\n');
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.includes('println!') || line.includes('eprintln!')) {
          console.error(`Error: Found println! or eprintln! in ${fullPath}:${i + 1}`);
          console.error(`  ${line.trim()}`);
          hasError = true;
        }
      }
    }
  }
  return hasError;
}

async function main() {
  const dirsToSearch = [];
  
  // Find all crates src dirs
  const cratesDir = path.join(process.cwd(), 'crates');
  const crates = await fs.readdir(cratesDir, { withFileTypes: true });
  for (const crate of crates) {
    if (crate.isDirectory()) {
      dirsToSearch.push(path.join(cratesDir, crate.name, 'src'));
    }
  }
  
  // Add src-tauri src
  dirsToSearch.push(path.join(process.cwd(), 'src-tauri', 'src'));

  let hasError = false;
  for (const dir of dirsToSearch) {
      try {
          const stat = await fs.stat(dir);
          if (stat.isDirectory()) {
              if (await searchDir(dir)) {
                  hasError = true;
              }
          }
      } catch (e) {
          // ignore if src doesn't exist
      }
  }

  if (hasError) {
    console.error('\nRuntime println!/eprintln! is forbidden. Use tracing::info!/error! instead.');
    process.exit(1);
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
