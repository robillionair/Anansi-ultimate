import type { Page } from '@playwright/test';
export function prepareLifecycle(page:Page,label:string):Promise<void>;
export function checkLifecycle(page:Page,label:string):Promise<void>;
