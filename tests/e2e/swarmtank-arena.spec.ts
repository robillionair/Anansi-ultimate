import { test, expect } from '@playwright/test';
import { collideAndSave, startSession } from './collider-three-helpers';
import { layoutSwarmtankArena } from '../../packages/ui/src/swarmtankArenaLayout';
import type {
  SwarmtankDecision,
  SwarmtankPersona,
} from '../../packages/contracts/src/generated';

/**
 * M9 SwarmTank Slice 2: the 3D arena over a saved
 * run. Three tests sharing one run builder (collide
 * → Build a real Forge concept → import the
 * experiment → Generate → Evaluate, the Slice 1
 * flow): (1) the canvas mounts, an agent click
 * raycasts to its popover card, and the table
 * synchronizes in both directions; (2) the
 * reduced-motion toggle renders a static view;
 * (3) without WebGL the arena falls back to its
 * note while the complete table stays intact. The
 * arena makes no provider calls at any step.
 *
 * Agent click math (documented in the Slice 2
 * design): the arena camera is fixed top-down at
 * height 640, fov 45, up (0, 0, -1), so NDC x =
 * x / (tan(22.5°) · 640 · aspect) and NDC y =
 * −z / (tan(22.5°) · 640), with aspect from the
 * canvas's actual box at click time.
 */

async function buildRun(page: import('@playwright/test').Page, reducedMotion: boolean) {
  page.setDefaultTimeout(8000);
  const session = await startSession(page, reducedMotion);
  const { workspaceId, projectId } = session;
  await collideAndSave(
    session.collider,
    'First beam about kitchens.',
    'First beam about orbitals.',
  );
  await session.artifacts
    .getByRole('button', { name: /Collider experiment: First beam about kitchens/ })
    .click();
  const build = session.artifacts.locator('section[aria-label="Build this product"]');
  await build.getByLabel('Combination to build').selectOption({ label: 'Mycelial order router' });
  await build.getByRole('button', { name: 'Build This Product' }).click();
  await expect(
    page.getByText('Built: Forge concept created from the approved outputs.'),
  ).toBeVisible();
  await session.openProject();

  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
  await expect(swarmtank.getByRole('heading', { name: 'SwarmTank', exact: true })).toBeVisible();
  await swarmtank
    .getByLabel('Import from saved experiment')
    .selectOption({ label: 'Collider experiment: First beam about kitchens.' });
  await swarmtank.getByRole('button', { name: 'Import experiment' }).click();
  await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
  await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
  await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
  await swarmtank.getByLabel('SwarmTank industry').fill('services');
  await swarmtank
    .getByLabel('SwarmTank value proposition')
    .fill('Agencies save ten hours a week on reporting.');
  await swarmtank.getByLabel('SwarmTank population').selectOption('20');
  await swarmtank.getByLabel('SwarmTank seed').fill('arena-seed-1');
  await swarmtank.getByRole('button', { name: 'Generate personas' }).click();
  await expect(swarmtank.getByRole('heading', { name: 'Simulated personas (preview, not saved)' })).toBeVisible();
  await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
  await expect(
    page.getByText('Evaluated and saved the SwarmTank run.'),
  ).toBeVisible();
  return { session, swarmtank, workspaceId, projectId };
}

async function readRun(session: Awaited<ReturnType<typeof startSession>>, workspaceId: string, projectId: string) {
  const listed = await session.call('artifacts', 'list', { workspaceId, projectId });
  expect(listed.status).toBe(200);
  const artifacts = listed.body.artifacts as Array<Record<string, unknown>>;
  const runArtifact = artifacts.find(artifact => artifact.type === 'simulation');
  expect(runArtifact).toBeTruthy();
  const read = await session.call('artifacts', 'read', {
    workspaceId,
    artifactId: runArtifact!.artifactId as string,
    version: runArtifact!.version as number,
  });
  expect(read.status).toBe(200);
  const envelope = read.body as { body?: unknown };
  const raw = envelope.body ?? read.body;
  const doc = (typeof raw === 'string' ? JSON.parse(raw) : raw) as {
    personas: Array<Record<string, unknown>>;
    decisions: Array<Record<string, unknown>>;
  };
  return doc;
}

test('arena mounts, agent click opens the popover, table syncs both ways', async ({ page }) => {
  test.setTimeout(300000);
  const { session, swarmtank, workspaceId, projectId } = await buildRun(page, false);
  try {
    const arena = swarmtank.locator('section[aria-label="SwarmTank 3D arena"]');
    await expect(arena.getByRole('heading', { name: 'SwarmTank 3D arena' })).toBeVisible();
    const canvas = arena.locator('canvas[data-three-canvas]');
    await expect(canvas).toBeVisible();
    const hitsBefore = session.fakeHits();

    const run = await readRun(session, workspaceId, projectId);
    const layout = layoutSwarmtankArena(
      run.personas as unknown as SwarmtankPersona[],
      run.decisions as unknown as SwarmtankDecision[],
    );
    const convert = layout.agents.find(agent => agent.decision === 'Convert')!;
    const convertPersona = run.personas.find(p => p.id === convert.personaId)!;
    const convertDecision = run.decisions.find(d => d.personaId === convert.personaId)!;
    await canvas.scrollIntoViewIfNeeded();
    const box = (await canvas.boundingBox())!;
    const aspect = box.width / box.height;
    const tanHalf = Math.tan((45 / 2) * Math.PI / 180);
    const ndcX = convert.x / (tanHalf * 640 * aspect);
    const ndcY = -convert.z / (tanHalf * 640);
    await page.mouse.click(
      box.x + ((ndcX + 1) / 2) * box.width,
      box.y + ((1 - ndcY) / 2) * box.height,
    );
    const card = arena.locator('section[aria-label="Selected persona"]');
    await expect(card).toBeVisible();
    await expect(card.getByText(convertPersona.name as string)).toBeVisible();
    await expect(card.getByText(`Role: ${convertPersona.role as string}`)).toBeVisible();
    await expect(card.getByText(`Approximate budget: ${convertPersona.approximateBudget as number}`)).toBeVisible();
    await expect(card.getByText(`Perceived value: ${convertDecision.perceivedValue as number}`)).toBeVisible();
    // Table row for the picked agent is marked.
    await expect(
      swarmtank.locator('tr', { hasText: convertPersona.name as string }).getByText('Selected in arena.'),
    ).toBeVisible();
    // Reverse sync: a row button selects another agent.
    const other = layout.agents.find(agent => agent.personaId !== convert.personaId)!;
    const otherPersona = run.personas.find(p => p.id === other.personaId)!;
    const otherDecision = run.decisions.find(d => d.personaId === other.personaId)!;
    await swarmtank
      .locator('tr', { hasText: otherPersona.name as string })
      .getByRole('button', { name: `Select ${otherPersona.name as string} in arena` })
      .click();
    await expect(card.getByText(otherPersona.name as string)).toBeVisible();
    await expect(card.getByText(`Key objection: ${String(otherDecision.keyObjection ?? 'None — converted.')}`)).toBeVisible();
    // The arena produced no provider traffic.
    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});

test('reduced-motion toggle renders a static arena view', async ({ page }) => {
  test.setTimeout(300000);
  const { session, swarmtank } = await buildRun(page, true);
  try {
    const arena = swarmtank.locator('section[aria-label="SwarmTank 3D arena"]');
    await expect(arena.getByRole('heading', { name: 'SwarmTank 3D arena' })).toBeVisible();
    const toggle = arena.getByLabel('Reduce motion (static arena view)');
    await expect(toggle).toBeChecked();
    await expect(arena.getByText('Motion reduced — static view.')).toBeVisible();
    await toggle.uncheck();
    await expect(arena.getByText('Motion reduced — static view.')).toHaveCount(0);
    await expect(arena.locator('canvas[data-three-canvas]')).toBeVisible();
    await toggle.check();
    await expect(arena.getByText('Motion reduced — static view.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('without WebGL the arena falls back to its note and the full table', async ({ page }) => {
  test.setTimeout(300000);
  // Plain-JS string: addInitScript evaluates the source
  // in the page, so TypeScript annotations here would
  // be syntax errors in the browser.
  await page.addInitScript(`
    const originalGetContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type) {
      if (typeof type === 'string' && type.indexOf('webgl') === 0) return null;
      return originalGetContext.apply(this, arguments);
    };
  `);
  const { session, swarmtank } = await buildRun(page, false);
  try {
    const arena = swarmtank.locator('section[aria-label="SwarmTank 3D arena"]');
    await expect(arena.getByRole('heading', { name: 'SwarmTank 3D arena' })).toBeVisible();
    await expect(
      arena.getByText('3D arena unavailable — the complete table below carries the same information.'),
    ).toBeVisible();
    // The complete table is still rendered and searchable, and the
    // baseline report metrics are visible. The baseline line is matched
    // by shape (integer percent + "(N of M)") so the Slice 3 scenario
    // metrics bar ("Scenario conversion rate: …") and comparison
    // ("Conversion rate: 100.00% baseline → …") do not collide with it.
    await expect(swarmtank.locator('section[aria-label="SwarmTank report"] table')).toBeVisible();
    await expect(swarmtank.getByText(/^Conversion rate: [0-9.]+% \([0-9]+ of/)).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
