// Recovery UI interaction coverage against a typed mock API.
//
// The exhausted state is unreachable in the real browser (claim
// faults come from the test-only Rust seam), so this spec serves
// the real app a diagnosed job from an in-process mock and proves
// the real interaction: the message renders, Retry recovery sends
// exactly one control call with action `recover` and the observed
// version, and the UI updates to the recovered job. The mock
// responses are typed with the generated contract interfaces, so
// tsc checks their shapes and the client's Ajv validators check
// them again at runtime.

import { test, expect } from '@playwright/test';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import type {
  BackupSchedule,
  Job,
  JobList,
  MemoryList,
  Readiness,
  RelationshipList,
  WorkspaceDetails,
  WorkspaceList,
} from '../../packages/contracts/src/generated.ts';

test.setTimeout(120000);

const WORKSPACE_ID = '73a8ff8000581853623bf6fc4733195b';
const JOB_ID = '07d43a3809f34c6c48c2759167b570dd';
const DIAGNOSTIC_MESSAGE =
  'Recovery stopped because storage was unavailable. Check the storage folder and permissions, then choose Retry recovery.';

const diagnosedJob: Job = {
  artifactBytes: null,
  artifactHash: null,
  artifactId: null,
  checkpoint: 'staged',
  checkpointVersion: 2,
  createdAt: '2026-11-12T00:04:10.469Z',
  id: JOB_ID,
  log: 'Upload complete; queued for import',
  maxBytes: 268435456,
  maxRecords: 10000,
  progress: 100,
  recoveryDiagnostic: {
    attempts: 3,
    message: DIAGNOSTIC_MESSAGE,
    reason: 'storageUnavailable',
  },
  selection: {
    expectedBytes: 54,
    expectedHash: 'f5d0aa1d545720634a9f94a12801b2ddb956aa2885edc9a9b6dd2f0333fc4879',
    fileName: 'poll.txt',
    format: 'txt',
    kind: 'ingestion',
    privacy: 'private',
    projectId: null,
    uploadId: JOB_ID,
  },
  state: 'Queued',
  updatedAt: '2026-11-12T00:04:11.099Z',
  version: 2,
  workspaceId: WORKSPACE_ID,
};

// After a successful Recover control, the real backend bumps the
// version, keeps the job Queued, and clears the exhaustion — so
// the diagnostic disappears from later reads.
const recoveredJob: Job = {
  ...diagnosedJob,
  recoveryDiagnostic: undefined,
  updatedAt: '2026-11-12T00:05:00.000Z',
  version: 3,
};

function workspaceDetails(name: string): WorkspaceDetails {
  return {
    contractVersion: 1,
    schemaVersion: 1,
    settings: { intent: 'explore', onboardingComplete: true, reducedMotion: false },
    workspace: { id: WORKSPACE_ID, name },
  };
}

const readiness: Readiness = {
  applicationName: 'ANANSI',
  applicationVersion: '0.1.0',
  contractVersion: 1,
  coreVersion: '0.1.0',
  operatingSystem: 'mock',
  uptimeMs: 1,
};

test('Retry recovery sends the exact control call and clears the notice', async ({ page }) => {
  page.setDefaultTimeout(8000);
  let recovered = false;
  const controlCalls: Array<Record<string, unknown>> = [];
  const unknownPaths: string[] = [];
  const server = http.createServer((request, response) => {
    // Mirror the real API's CORS boundary: the preview origin is
    // the only allowed origin, preflight permits POST with the
    // authorization/content-type headers and discloses no data.
    response.setHeader('Access-Control-Allow-Origin', 'http://127.0.0.1:1420');
    if (request.method === 'OPTIONS') {
      response.writeHead(204, {
        'Access-Control-Allow-Methods': 'POST',
        'Access-Control-Allow-Headers': 'authorization, content-type',
      });
      response.end();
      return;
    }
    const chunks: Buffer[] = [];
    request.on('data', chunk => chunks.push(chunk as Buffer));
    request.on('end', () => {
      const body = chunks.length ? JSON.parse(Buffer.concat(chunks).toString('utf8')) : {};
      const path = request.url ?? '';
      let payload: unknown;
      if (path === '/v1/readiness') payload = readiness;
      else if (path === '/v1/workspaces/list') payload = { contractVersion: 1, workspaces: [] } satisfies WorkspaceList;
      else if (path === '/v1/workspaces/create') payload = workspaceDetails(String(body.name ?? 'Mock workspace'));
      else if (path === '/v1/workspaces/settings') payload = workspaceDetails('Mock workspace');
      else if (path === '/v1/jobs/list') {
        payload = {
          contractVersion: 1,
          jobs: [recovered ? recoveredJob : diagnosedJob],
          nextOffset: null,
        } satisfies JobList;
      } else if (path === '/v1/jobs/control') {
        controlCalls.push(body as Record<string, unknown>);
        recovered = true;
        payload = { contractVersion: 1, job: recoveredJob };
      } else if (path === '/v1/memories/list') {
        payload = { contractVersion: 1, memories: [], nextOffset: null } satisfies MemoryList;
      } else if (path === '/v1/relationships/list') {
        payload = { contractVersion: 1, relationships: [] } satisfies RelationshipList;
      } else if (path === '/v1/jobs/schedule-read') {
        payload = {
          contractVersion: 1,
          destination: '',
          enabled: false,
          intervalSeconds: 0,
          missedRuns: 0,
          nextDue: null,
          retention: 'latest',
        } satisfies BackupSchedule;
      } else {
        unknownPaths.push(path);
        response.writeHead(404, { 'Content-Type': 'application/json' });
        response.end(JSON.stringify({ code: 'NOT_FOUND', message: `unknown mock path ${path}` }));
        return;
      }
      response.writeHead(200, { 'Content-Type': 'application/json' });
      response.end(JSON.stringify(payload));
    });
  });
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = (server.address() as AddressInfo).port;
  try {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(`http://127.0.0.1:${port}`);
    await page.getByLabel('Session token').fill('a'.repeat(64));
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('heading', { name: 'Choose your starting point' })).toBeVisible();
    await page.getByRole('button', { name: 'Continue to workspace' }).click();
    await page.getByLabel('New workspace name').fill('Mock workspace');
    await page.getByRole('button', { name: 'Create workspace' }).click();
    await page.getByRole('button', { name: 'Jobs and export' }).click();
    await expect(page.getByRole('heading', { name: 'Your jobs and exports' })).toBeVisible();

    const alert = page.getByRole('alert');
    await expect(alert).toContainText(DIAGNOSTIC_MESSAGE);
    await expect(alert).toContainText('Attempts used: 3.');
    const retry = page.getByRole('button', { name: 'Retry recovery' });
    await expect(retry).toBeVisible();
    await retry.click();

    await expect
      .poll(() => controlCalls.length, { timeout: 8000 })
      .toBe(1);
    expect(controlCalls[0]).toEqual({
      contractVersion: 1,
      workspaceId: WORKSPACE_ID,
      id: JOB_ID,
      expectedVersion: 2,
      action: 'recover',
    });
    await expect(alert).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Retry recovery' })).toHaveCount(0);
    await expect(page.getByText('State:')).toBeVisible();
    expect(unknownPaths).toEqual([]);
  } finally {
    await new Promise<void>(resolve => server.close(() => resolve()));
  }
});
