import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Windows file events can miss rapid edits and leave cached API modules stale.
    watch: { usePolling: true, interval: 300, ignored: ['**/src-tauri/**', '**/env/**', '**/release/**'] },
  },
  build: { target: 'es2022' },
});
