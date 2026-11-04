import { defineConfig } from 'vite';

// The screenshots live in ../docs, so the dev server may read one level up.
export default defineConfig({
  base: './',
  server: { fs: { allow: ['..'] } },
  build: { outDir: 'dist', assetsInlineLimit: 0, chunkSizeWarningLimit: 1200 },
});
