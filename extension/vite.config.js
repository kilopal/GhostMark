import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    target: 'esnext',
    outDir: 'dist',
    rollupOptions: {
      input: {
        offscreen: resolve(__dirname, 'offscreen.js')
      },
      output: {
        entryFileNames: '[name].bundle.js'
      }
    }
  }
});
