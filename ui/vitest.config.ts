import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['tests/**/*.test.{ts,mjs}'],
    setupFiles: ['tests/setup.mjs'],
    coverage: {
      provider: 'v8',
      include: ['src/**/*.{ts,tsx}'],
      reporter: ['text', 'html', 'json', 'lcov'],
      reportsDirectory: '../build/coverage/ui',
      reportOnFailure: true,
    },
  },
});
