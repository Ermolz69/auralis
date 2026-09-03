import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export async function serveProduction() {
  const root = fileURLToPath(new URL('../dist/', import.meta.url));
  const config = JSON.parse(
    await readFile(new URL('../../../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );
  const csp = Object.entries(config.app.security.csp)
    .map(([key, value]) => `${key} ${value}`)
    .join('; ');
  const types = {
    '.html': 'text/html',
    '.js': 'text/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
  };
  const server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://localhost').pathname;
      const target = path.resolve(
        root,
        '.' + decodeURIComponent(pathname === '/' ? '/index.html' : pathname),
      );
      if (!target.startsWith(root)) throw new Error('Path outside dist');
      const content = await readFile(target);
      response.writeHead(200, {
        'Content-Type': types[path.extname(target)] ?? 'application/octet-stream',
        'Content-Security-Policy': csp,
      });
      response.end(content);
    } catch {
      if (!response.headersSent) response.writeHead(404);
      response.end();
    }
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  return {
    url: `http://127.0.0.1:${server.address().port}`,
    close: () =>
      new Promise((resolve) => {
        server.closeAllConnections();
        server.close(resolve);
      }),
  };
}
