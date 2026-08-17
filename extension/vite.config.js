import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    target: 'esnext',
    outDir: 'dist',
    rollupOptions: {
      input: {
        popup: resolve(__dirname, 'popup.js')
      },
      output: {
        entryFileNames: '[name].bundle.js'
      }
    }
  }
});
