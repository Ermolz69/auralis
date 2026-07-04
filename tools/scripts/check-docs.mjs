import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');

const REQUIRED_DOCS = [
  'docs/README.md',
  'docs/architecture/000-stack.md',
  'docs/architecture/001-overview.md',
  'docs/architecture/002-frontend-fsd.md',
  'docs/architecture/003-rust-workspace.md',
  'docs/ci/001-quality-gates.md',
  'docs/taskfile/001-commands.md'
];

let hasErrors = false;

for (const doc of REQUIRED_DOCS) {
  const fullPath = path.join(rootDir, doc);
  if (!fs.existsSync(fullPath)) {
    console.error(`ERROR: Missing mandatory documentation file: ${doc}`);
    hasErrors = true;
  }
}

if (hasErrors) {
  console.error('\nArchitecture and core processes must not remain "in the head".');
  console.error('Please create the missing documentation files to pass this quality gate.');
  process.exit(1);
} else {
  console.log("SUCCESS: All mandatory documentation files are present.");
}
