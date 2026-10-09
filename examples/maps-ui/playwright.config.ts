import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './browser-tests', workers: 1,
  use: { baseURL: 'http://127.0.0.1:55467', browserName: 'chromium', launchOptions: { executablePath: process.env.ROM_CHROMIUM_PATH, args: ['--enable-unsafe-swiftshader', '--use-angle=swiftshader-webgl'] } },
  webServer: { command: 'corepack pnpm exec vite preview --host 127.0.0.1 --port 55467 --strictPort', url: 'http://127.0.0.1:55467', reuseExistingServer: false },
});
