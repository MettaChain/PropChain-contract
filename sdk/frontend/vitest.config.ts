import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['__tests__/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.ts'],
      exclude: ['src/abi/**', 'src/**/*.d.ts'],
    },
    testTimeout: 30000,
    setupFiles: [],
  },
  resolve: {
    alias: {
      '@': './src',
    },
  },
});
