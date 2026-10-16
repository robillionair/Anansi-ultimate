import { defineConfig } from '@playwright/test';
export default defineConfig({testDir:'./tests/e2e',workers:1,fullyParallel:false,reporter:'list',use:{baseURL:'http://127.0.0.1:1420',trace:'off',screenshot:'off'},webServer:{command:'pnpm --filter @anansi/web preview',url:'http://127.0.0.1:1420',reuseExistingServer:false,timeout:30000}});
