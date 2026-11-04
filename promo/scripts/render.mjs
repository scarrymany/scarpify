// Renders the promo frame by frame: serves dist/, opens it in headless Chromium with
// ?render, asks the page for each frame at a fixed clock (frame / 60 s), and pipes the raw
// pixels into ffmpeg. Nothing depends on wall-clock time, so every run gives the same video.
//
//   node scripts/render.mjs                    -> out/scarpify-promo.mp4
//   node scripts/render.mjs --stills 2,9,14    -> out/stills/frame-02.0s.png, ...
//   node scripts/render.mjs --from 10 --to 12  -> a partial render, for checking a moment

import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { extname, join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer-core';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(root, 'dist');
const out = join(root, 'out');

const args = process.argv.slice(2);
const opt = (name) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : undefined;
};

function findChrome() {
  const candidates = [
    process.env.CHROME_PATH,
    '/opt/pw-browsers/chromium-1194/chrome-linux/chrome',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/usr/bin/google-chrome',
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  ].filter(Boolean);
  const found = candidates.find((p) => existsSync(p));
  if (!found) throw new Error('No Chrome or Chromium found; set CHROME_PATH.');
  return found;
}

const types = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.png': 'image/png',
  '.woff2': 'font/woff2',
  '.woff': 'font/woff',
};

function serve() {
  const server = createServer(async (req, res) => {
    const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    const file = join(dist, path === '/' ? 'index.html' : path);
    if (!file.startsWith(dist)) return res.writeHead(403).end();
    try {
      const body = await readFile(file);
      res.writeHead(200, { 'content-type': types[extname(file)] ?? 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  return new Promise((ok) => server.listen(0, '127.0.0.1', () => ok(server)));
}

function ffmpeg(argv) {
  const p = spawn('ffmpeg', argv, { stdio: ['pipe', 'inherit', 'inherit'] });
  const done = new Promise((ok, fail) => p.on('close', (code) => (code === 0 ? ok() : fail(new Error(`ffmpeg exited ${code}`)))));
  return { stdin: p.stdin, done };
}

function write(stream, buf) {
  return new Promise((ok) => (stream.write(buf) ? ok() : stream.once('drain', ok)));
}

const server = await serve();
const url = `http://127.0.0.1:${server.address().port}/?render`;
const browser = await puppeteer.launch({
  executablePath: findChrome(),
  headless: true,
  args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--hide-scrollbars'],
  defaultViewport: { width: 1920, height: 1080, deviceScaleFactor: 1 },
});

try {
  const page = await browser.newPage();
  page.on('console', (m) => (m.type() === 'error' || m.type() === 'warn') && console.log('[page]', m.text()));
  page.on('pageerror', (e) => console.error('[page]', e));
  await page.goto(url, { waitUntil: 'load' });
  await page.waitForFunction('window.__promo');
  await page.evaluate('window.__promo.ready');
  const { fps, duration, width, height } = await page.evaluate(() => {
    const { fps, duration, width, height } = window.__promo;
    return { fps, duration, width, height };
  });
  const raw = ['-f', 'rawvideo', '-pix_fmt', 'rgba', '-s', `${width}x${height}`];

  const stills = opt('stills');
  if (stills) {
    await mkdir(join(out, 'stills'), { recursive: true });
    for (const s of stills.split(',').map(Number)) {
      const i = Math.round(s * fps);
      const b64 = await page.evaluate((i) => window.__promo.frame(i), i);
      const name = join(out, 'stills', `frame-${s.toFixed(1).padStart(4, '0')}s.png`);
      const f = ffmpeg(['-y', '-loglevel', 'error', ...raw, '-i', '-', '-vf', 'vflip', '-frames:v', '1', name]);
      f.stdin.end(Buffer.from(b64, 'base64'));
      await f.done;
      console.log(name);
    }
  } else {
    const from = Math.round(Number(opt('from') ?? 0) * fps);
    const to = Math.round(Number(opt('to') ?? duration) * fps);
    const file = opt('out') ?? join(out, 'scarpify-promo.mp4');
    await mkdir(dirname(file), { recursive: true });
    const f = ffmpeg([
      '-y', '-loglevel', 'error', ...raw, '-r', String(fps), '-i', '-',
      '-vf', 'vflip,scale=out_color_matrix=bt709:out_range=tv,format=yuv420p', '-c:v', 'libx264', '-preset', 'slow', '-crf', '16', '-tune', 'film',
      '-pix_fmt', 'yuv420p', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-colorspace', 'bt709',
      '-movflags', '+faststart', file,
    ]);
    const started = Date.now();
    for (let i = from; i < to; i++) {
      const b64 = await page.evaluate((i) => window.__promo.frame(i), i);
      await write(f.stdin, Buffer.from(b64, 'base64'));
      if ((i - from) % 30 === 0 || i === to - 1) {
        const done = i - from + 1;
        const eta = ((Date.now() - started) / done) * (to - from - done) / 1000;
        process.stdout.write(`\rframe ${i + 1}/${to}  eta ${Math.round(eta)}s   `);
      }
    }
    f.stdin.end();
    await f.done;
    console.log(`\n${file}`);
  }
} finally {
  await browser.close();
  server.close();
}
