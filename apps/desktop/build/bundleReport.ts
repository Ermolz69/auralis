import path from 'node:path';
import type { Plugin } from 'vite';

export function bundleReport(): Plugin {
  let root = '';
  return {
    name: 'auralis-bundle-report',
    apply: 'build',
    configResolved(config) {
      root = config.root;
    },
    generateBundle(_options, bundle) {
      const chunks = Object.values(bundle)
        .filter((item) => item.type === 'chunk')
        .map((chunk) => ({
          file: chunk.fileName,
          entry: chunk.isEntry,
          imports: chunk.imports,
          dynamicImports: chunk.dynamicImports,
          bytes: Buffer.byteLength(chunk.code),
          modules: Object.entries(chunk.modules)
            .map(([id, info]) => ({
              id: path.relative(root, id).replaceAll('\\', '/'),
              renderedBytes: info.renderedLength,
            }))
            .sort((a, b) => b.renderedBytes - a.renderedBytes),
        }))
        .sort((a, b) => b.bytes - a.bytes);
      this.emitFile({
        type: 'asset',
        fileName: 'bundle-report.json',
        source: JSON.stringify(chunks, null, 2),
      });
    },
  };
}
