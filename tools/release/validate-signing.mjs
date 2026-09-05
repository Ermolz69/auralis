import { validateReleaseSigning } from './signing-config.mjs';

const platform = process.argv.find((argument) => argument.startsWith('--platform='))?.split('=')[1];
if (!platform) {
  process.stderr.write('Pass --platform=windows, --platform=macos or --platform=linux\n');
  process.exitCode = 1;
} else {
  try {
    validateReleaseSigning(platform);
    process.stdout.write(`Release signing configuration is complete for ${platform}.\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
