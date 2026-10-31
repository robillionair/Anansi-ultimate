import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import { collideAndSave, startSession } from './collider-three-helpers';
import { layoutSwarmtankArena } from '../../packages/ui/src/swarmtankArenaLayout';
import {
  evaluateScenarioPopulation,
} from '../../packages/ui/src/swarmtankSensitivity';
import type {
  SwarmtankDecision,
  SwarmtankInput,
  SwarmtankPersona,
  SwarmtankScenario,
} from '../../packages/contracts/src/generated';

/**
 * M9 SwarmTank Slice 3: interactive scenario
 * controls over a saved run. Three tests sharing
 * the Slice 2 run builder (collide → Build a real
 * Forge concept → import the experiment → Generate
 * → Evaluate): (1) price, feature, and setup
 * controls drive the metrics bar, the comparison
 * studio, and the arena re-clustering — the arena
 * counts match the computed scenario counts, and a
 * canvas click on a computed scenario agent opens
 * its scenario decision; (2) a data-driven lever
 * (free trial resolving a hesitation, or a feature
 * drop downgrading a convert) moves the comparison;
 * (3) a saved branch feeds a Forge revision to v3
 * carrying the scenario diff, and both the run and
 * the branch survive a fresh backup/restore. No
 * provider traffic at any step.
 *
 * Score monotonicity note for test 1: raising the
 * price, dropping features, and choosing complex
 * setup can only lower (never raise) a persona's
 * factors, so the scenario conversion rate is
 * guaranteed <= the baseline rate — but not
 * strictly lower, so the test asserts the bound,
 * not a drop.
 */

function sha256(data: Buffer): string {
  return createHash('sha256').update(data).digest('hex');
}

async function buildRun(page: import('@playwright/test').Page) {
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
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
  await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
  await expect(
    page.getByText('Evaluated and saved the SwarmTank run.'),
  ).toBeVisible();
  return { session, swarmtank, workspaceId, projectId };
}

type RunDoc = {
  input: SwarmtankInput;
  personas: SwarmtankPersona[];
  decisions: SwarmtankDecision[];
};

async function readRun(
  session: Awaited<ReturnType<typeof startSession>>,
  workspaceId: string,
  projectId: string,
): Promise<{ run: RunDoc; artifact: Record<string, unknown> }> {
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
  const doc = (typeof raw === 'string' ? JSON.parse(raw) : raw) as RunDoc;
  return { run: doc, artifact: runArtifact! };
}

async function listArtifacts(
  session: Awaited<ReturnType<typeof startSession>>,
  workspaceId: string,
  projectId: string,
) {
  const listed = await session.call('artifacts', 'list', { workspaceId, projectId });
  expect(listed.status).toBe(200);
  return listed.body.artifacts as Array<Record<string, unknown>>;
}

async function readArtifactBody(
  session: Awaited<ReturnType<typeof startSession>>,
  workspaceId: string,
  artifactId: string,
  version: number,
): Promise<Record<string, unknown>> {
  const read = await session.call('artifacts', 'read', {
    workspaceId,
    artifactId,
    version,
  });
  expect(read.status).toBe(200);
  const envelope = read.body as { body?: unknown };
  const raw = envelope.body ?? read.body;
  return (typeof raw === 'string' ? JSON.parse(raw) : raw) as Record<string, unknown>;
}

test('price slider updates conversion and the 3D arena re-clusters', async ({ page }) => {
  test.setTimeout(300000);
  const { session, swarmtank, workspaceId, projectId } = await buildRun(page);
  try {
    const hitsBefore = session.fakeHits();
    const report = swarmtank.locator('section[aria-label="SwarmTank report"]');
    const controls = swarmtank.locator('section[aria-label="SwarmTank scenario controls"]');
    const comparison = swarmtank.locator('section[aria-label="SwarmTank scenario comparison"]');
    const arena = swarmtank.locator('section[aria-label="SwarmTank 3D arena"]');
    await expect(controls.getByRole('heading', { name: 'Scenario controls' })).toBeVisible();
    await expect(controls.getByText('⚡ Instant Synthetic Sensitivity Calculation')).toBeVisible();

    const baselineText = (await report.getByText(/Conversion rate: \d+%/).first().innerText())!;
    const baselineRate = Number(/Conversion rate: (\d+(\.\d+)?)%/.exec(baselineText)![1]);
    expect(baselineRate).toBeGreaterThan(0);

    // Harsh scenario: top price the slider allows,
    // every feature off, complex setup.
    await controls.getByLabel('Scenario monthly price (exact value)').fill('490');
    const boxes = controls.locator('input[type="checkbox"]');
    const boxCount = await boxes.count();
    for (let i = 0; i < boxCount; i++) {
      if (await boxes.nth(i).isChecked()) await boxes.nth(i).uncheck();
    }
    await controls.getByRole('radio', { name: 'complex' }).check();
    // Let the 600 ms re-clustering transition settle.
    await page.waitForTimeout(1200);

    // The comparison reflects the driven state.
    await expect(comparison.getByText('Scenario price: $490')).toBeVisible();
    await expect(comparison.getByText(/active features: 0;/)).toBeVisible();
    await expect(comparison.getByText(/setup: complex/)).toBeVisible();
    const metricsText = (await swarmtank.getByText(/Scenario conversion rate: \d+(\.\d+)?%/).innerText())!;
    const scenarioRate = Number(/Scenario conversion rate: (\d+(\.\d+)?)%/.exec(metricsText)![1]);
    expect(scenarioRate).toBeLessThanOrEqual(baselineRate);

    // The arena counts match the computed scenario
    // counts: re-clustering follows scenario decisions.
    const { run } = await readRun(session, workspaceId, projectId);
    const scenarioDecisions = evaluateScenarioPopulation(
      run.personas,
      run.input,
      run.decisions,
      {
        priceMonthly: 490,
        activeFeatures: [],
        trustSignals: {
          soc2Security: false,
          freeTrialAvailable: false,
          slaGuarantee: false,
          moneyBackGuarantee: false,
        },
        setupDifficulty: 'complex',
      } as SwarmtankScenario,
    );
    const scenarioConverts = scenarioDecisions.filter(d => d.decision === 'Convert').length;
    const scenarioConfused = scenarioDecisions.filter(d => d.decision === 'Confused').length;
    const scenarioRejects = scenarioDecisions.filter(d => d.decision === 'Reject').length;
    const countsText = (await arena.getByText(/Convert \(green/).innerText())!;
    expect(countsText).toContain(`${scenarioConverts} Convert (green`);
    expect(countsText).toContain(`${scenarioConfused} Confused (amber`);
    expect(countsText).toContain(`${scenarioRejects} Reject (red`);

    // Click a static (non-orbiting) scenario agent
    // and check the card shows its scenario decision.
    const layout = layoutSwarmtankArena(run.personas, scenarioDecisions);
    const target = layout.agents.find(agent => agent.decision !== 'Confused');
    expect(target).toBeTruthy();
    const targetPersona = run.personas.find(p => p.id === target!.personaId)!;
    const targetDecision = scenarioDecisions.find(d => d.personaId === target!.personaId)!;
    const canvas = arena.locator('canvas[data-three-canvas]');
    await expect(canvas).toBeVisible();
    await canvas.scrollIntoViewIfNeeded();
    const box = (await canvas.boundingBox())!;
    const aspect = box.width / box.height;
    const tanHalf = Math.tan((45 / 2) * Math.PI / 180);
    const ndcX = target!.x / (tanHalf * 640 * aspect);
    const ndcY = -target!.z / (tanHalf * 640);
    await page.mouse.click(
      box.x + ((ndcX + 1) / 2) * box.width,
      box.y + ((1 - ndcY) / 2) * box.height,
    );
    const card = arena.locator('section[aria-label="Selected persona"]');
    await expect(card).toBeVisible();
    await expect(card.getByText(targetPersona.name)).toBeVisible();
    await expect(card.getByText(`Decision: ${targetDecision.decision}`)).toBeVisible();

    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});

test('feature toggles and trust signals resolve objections', async ({ page }) => {
  test.setTimeout(300000);
  const { session, swarmtank, workspaceId, projectId } = await buildRun(page);
  try {
    const hitsBefore = session.fakeHits();
    const controls = swarmtank.locator('section[aria-label="SwarmTank scenario controls"]');
    const comparison = swarmtank.locator('section[aria-label="SwarmTank scenario comparison"]');
    await expect(controls.getByRole('heading', { name: 'Scenario controls' })).toBeVisible();

    const { run } = await readRun(session, workspaceId, projectId);
    const decisionById = new Map(run.decisions.map(d => [d.personaId, d]));
    // Lever A: a Confused freelancer/agency persona
    // whose hesitation a free trial resolves.
    const trialCandidate = run.personas.find(persona => {
      const decision = decisionById.get(persona.id);
      return (
        decision?.decision === 'Confused' &&
        (persona.role === 'Cost-sensitive freelancer' ||
          persona.role === 'Small agency owner') &&
        (decision.keyObjection === 'Price sensitivity' ||
          decision.keyObjection === 'Missing trust signals')
      );
    });
    if (trialCandidate) {
      const objection = decisionById.get(trialCandidate.id)!.keyObjection!;
      const escapeRegExp = (text: string) =>
        text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      await controls.getByLabel('Free trial available').check();
      await expect(
        comparison.getByText(new RegExp(`Converted \\(\\d+\\):[^.]*${escapeRegExp(trialCandidate.name)}`)),
      ).toBeVisible();
      await expect(
        comparison.getByText(new RegExp(`Resolved:[^.]*${escapeRegExp(objection)}`)),
      ).toBeVisible();
    } else {
      // Lever B: a Convert that loses a covered need
      // when one feature is toggled off. The triple
      // is verified against the engine in-test first,
      // so the UI assertion matches the computed
      // outcome exactly (a need can also be covered
      // by the value proposition, which no toggle
      // removes).
      const escapeRegExp = (text: string) =>
        text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      let lever: {
        victim: SwarmtankPersona;
        feature: string;
        objection: string;
      } | null = null;
      for (const persona of run.personas) {
        if (lever) break;
        if (decisionById.get(persona.id)?.decision !== 'Convert') continue;
        for (const feature of run.input.features ?? []) {
          const scenarioDecisions = evaluateScenarioPopulation(
            run.personas,
            run.input,
            run.decisions,
            {
              priceMonthly: null,
              activeFeatures: (run.input.features ?? []).filter(f => f !== feature),
              trustSignals: {
                soc2Security: false,
                freeTrialAvailable: false,
                slaGuarantee: false,
                moneyBackGuarantee: false,
              },
              setupDifficulty: 'moderate',
            } as SwarmtankScenario,
          );
          const outcome = scenarioDecisions.find(d => d.personaId === persona.id)!;
          if (
            outcome.decision === 'Confused' &&
            (outcome.keyObjection ?? '').startsWith(
              'Missing critical required feature: ',
            )
          ) {
            lever = { victim: persona, feature, objection: outcome.keyObjection! };
            break;
          }
        }
      }
      expect(lever).toBeTruthy();
      await controls.getByLabel(lever!.feature, { exact: true }).uncheck();
      await expect(
        comparison.getByText(new RegExp(`Lost \\(\\d+\\):[^.]*${escapeRegExp(lever!.victim.name)}`)),
      ).toBeVisible();
      await expect(
        comparison.getByText(`Introduced: ${lever!.objection}`),
      ).toBeVisible();
    }
    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});

test('scenario branching, Forge revision to v3 with scenario diff, and backup/restore', async ({ page }) => {
  test.setTimeout(300000);
  const { session, swarmtank, workspaceId, projectId } = await buildRun(page);
  try {
    const hitsBefore = session.fakeHits();
    const report = swarmtank.locator('section[aria-label="SwarmTank report"]');
    await expect(report.getByRole('heading', { name: 'SwarmTank report' })).toBeVisible();

    // Baseline transfer first: concept v2.
    await swarmtank
      .getByLabel('Apply recommendations to Forge concept')
      .selectOption({ label: 'Concept: Mycelial order router' });
    await swarmtank
      .getByRole('button', { name: 'Apply Recommendations to Forge' })
      .click();
    await expect(
      page.getByText(/Applied: Forge concept revised to version 2 with the SwarmTank revision note/),
    ).toBeVisible();

    // Scenario: price 99, then save the branch.
    const controls = swarmtank.locator('section[aria-label="SwarmTank scenario controls"]');
    await controls.getByLabel('Scenario monthly price (exact value)').fill('99');
    await swarmtank.getByRole('button', { name: 'Save Scenario as Branch' }).click();
    await expect(page.getByText(/Saved the scenario as a branch/)).toBeVisible();

    // Scenario transfer: concept v3 with the diff.
    await swarmtank.getByRole('radio', { name: 'Apply Scenario Recommendations to Forge' }).check();
    await swarmtank
      .getByRole('button', { name: 'Apply Scenario Recommendations to Forge' })
      .click();
    await expect(
      page.getByText(/Applied: Forge concept revised to version 3 with the SwarmTank revision note/),
    ).toBeVisible();

    const artifacts = await listArtifacts(session, workspaceId, projectId);
    const concept = artifacts.find(artifact => artifact.type === 'concept')!;
    expect(concept.version).toBe(3);
    const conceptDoc = await readArtifactBody(
      session,
      workspaceId,
      concept.artifactId as string,
      3,
    );
    expect(conceptDoc.swarmtankAddendum).toBeTruthy();
    const diff = conceptDoc.swarmtankScenarioDiff as Record<string, unknown>;
    expect(diff).toBeTruthy();
    expect(typeof diff.baselineConversionRate).toBe('number');
    expect(typeof diff.scenarioConversionRate).toBe('number');
    expect(typeof diff.optimalPriceRecommendation).toBe('string');
    expect((diff.optimalPriceRecommendation as string).length).toBeGreaterThan(0);

    const branch = artifacts.find(artifact => artifact.type === 'scenario')!;
    const branchDoc = await readArtifactBody(
      session,
      workspaceId,
      branch.artifactId as string,
      branch.version as number,
    );
    expect(branchDoc.format).toBe('anansi-swarmtank-scenario');
    expect(branchDoc.version).toBe(1);
    expect((branchDoc.decisions as unknown[]).length).toBe(20);
    expect(diff.scenarioArtifactId).toBe(branch.artifactId);
    expect(diff.scenarioVersion).toBe(branch.version);
    const { artifact: runArtifact } = await readRun(session, workspaceId, projectId);
    expect(branchDoc.parentArtifactId).toBe(runArtifact.artifactId);
    expect(branchDoc.parentVersion).toBe(runArtifact.version);

    // Backup, fresh restore, reopen: the run and
    // the branch both survive with content intact.
    const backup = await session.call('jobs', 'backup', {
      workspaceId,
      application: false,
      plaintextConsent: true,
    });
    expect(backup.status).toBe(200);
    const jobId = backup.body.job.id as string;
    let job: Record<string, any> | undefined;
    for (let i = 0; i < 60 && (!job || job.state !== 'Completed'); i++) {
      await new Promise(r => setTimeout(r, 1000));
      const list = await session.call('jobs', 'list', { workspaceId });
      job = (list.body.jobs as Array<Record<string, any>>).find(j => j.id === jobId);
    }
    expect(job!.state).toBe('Completed');
    const artifactBytes = job!.artifactBytes as number;
    const chunks: number[] = [];
    for (let offset = 0; offset < artifactBytes;) {
      const chunk = await session.call('jobs', 'artifact', {
        workspaceId,
        jobId,
        artifactId: job!.artifactId as string,
        offset,
      });
      const bytes = chunk.body.bytes as number[];
      expect(bytes.length).toBeGreaterThan(0);
      chunks.push(...bytes);
      offset += bytes.length;
    }
    expect(chunks.length).toBe(artifactBytes);
    const tar = Buffer.from(chunks);
    const landing = await session.call('workspaces', 'create', { name: 'SwarmTank Scenario Landing' });
    const landingId = landing.body.workspace.id as string;
    const begin = await session.call('jobs', 'upload-begin', {
      workspaceId: landingId,
      bytes: tar.length,
      sha256: sha256(tar),
      plaintextConsent: true,
    });
    const uploadId = begin.body.id as string;
    let uploadArtifactId = '';
    let importJobId = '';
    for (let offset = 0; offset < tar.length; offset += 65536) {
      const end = Math.min(offset + 65536, tar.length);
      const part = await session.call('jobs', 'upload-part', {
        workspaceId: landingId,
        id: uploadId,
        offset,
        bytes: Array.from(tar.subarray(offset, end)),
        finish: end === tar.length,
      });
      if (part.body.job?.artifactId) uploadArtifactId = part.body.job.artifactId as string;
      if (part.body.job?.id) importJobId = part.body.job.id as string;
    }
    expect(uploadArtifactId).not.toBe('');
    expect(importJobId).not.toBe('');
    const previewRestore = await session.call('jobs', 'preview', {
      workspaceId: landingId,
      jobId: importJobId,
      artifactId: uploadArtifactId,
    });
    expect(previewRestore.status).toBe(200);
    const restored = await session.call('jobs', 'restore', {
      workspaceId: landingId,
      jobId: importJobId,
      artifactId: uploadArtifactId,
      name: 'SwarmTank Scenario Restored',
      expectedHash: previewRestore.body.sha256,
      confirm: true,
    });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await session.pairAndOpen(restoredWs);
    const restoredRuns = swarmtank.locator('section[aria-label="Saved SwarmTank runs"]');
    await expect(restoredRuns.getByText(/SwarmTank run: A routing advisor/)).toBeVisible();
    const restoredBranches = swarmtank.locator('section[aria-label="Saved SwarmTank scenario branches"]');
    await expect(restoredBranches).toBeVisible();
    await restoredBranches.getByRole('button', { name: 'View comparison' }).first().click();
    const restoredComparison = swarmtank.locator('section[aria-label="SwarmTank scenario comparison"]');
    await expect(restoredComparison).toBeVisible();
    await expect(restoredComparison.getByText('Scenario price: $99')).toBeVisible();

    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});
