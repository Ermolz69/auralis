import fs from 'fs/promises';
import path from 'path';
import url from 'url';

const IGNORED_DIRS = ['tests', 'examples', 'benches'];

export async function checkFileContent(fullPath, content) {
    let hasError = false;
    const lines = content.split('\n');
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.includes('println!') || line.includes('eprintln!') || line.includes('dbg!')) {
            console.error(`Error: Found println!, eprintln! or dbg! in ${fullPath}:${i + 1}`);
            console.error(`  ${line.trim()}`);
            hasError = true;
        }
    }
    return hasError;
}

export async function searchDir(dir) {
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
            if (await checkFileContent(fullPath, content)) {
                hasError = true;
            }
        }
    }
    return hasError;
}

export async function runChecks(cwd) {
    const dirsToSearch = [];
    
    // Find all crates src dirs
    const cratesDir = path.join(cwd, 'crates');
    try {
        const crates = await fs.readdir(cratesDir, { withFileTypes: true });
        for (const crate of crates) {
            if (crate.isDirectory()) {
                dirsToSearch.push(path.join(cratesDir, crate.name, 'src'));
            }
        }
    } catch (e) {
        if (e.code !== 'ENOENT') {
            console.error(`Failed to read crates directory: ${e.message}`);
            throw e;
        }
    }
    
    // Add src-tauri src
    dirsToSearch.push(path.join(cwd, 'src-tauri', 'src'));

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
            if (e.code === 'ENOENT') {
                continue; // src directory doesn't exist, which is fine
            }
            console.error(`Filesystem error accessing ${dir}: ${e.message}`);
            throw e;
        }
    }

    return hasError;
}

const isMain = process.argv[1] && import.meta.url === url.pathToFileURL(process.argv[1]).href;

if (isMain) {
    runChecks(process.cwd())
        .then(hasError => {
            if (hasError) {
                console.error('\nRuntime println!/eprintln!/dbg! is forbidden. Use tracing::info!/error! instead.');
                process.exit(1);
            }
        })
        .catch(err => {
            console.error(err);
            process.exit(1);
        });
}
