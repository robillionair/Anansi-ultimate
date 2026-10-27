import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 acceptance repair, independent-process atomicity:
 * two separate API processes share one synthetic
 * storage root. Both submit the identical transfer
 * concurrently (Promise.all, no sleeps, no retries);
 * the transactional receipt check must return the
 * same concept id from both and leave exactly one
 * concept artifact. A second, different brief is a
 * second transfer, not a silent merge.
 */
function experimentBody(): string {
  return JSON.stringify({
    format: 'anansi-collider-experiment',
    version: 1,
    beamA: 'Trading signals.',
    beamB: 'Mycelium networks.',
    controls: { creativity: 'balanced', technicalDepth: 'standard', practicality: 'balanced', innovationDistance: 'moderate', budget: '', preferredOutputType: 'concept' },
    result: {
      sourceUnderstanding: 'Both beams describe decentralized coordination systems with local feedback.',
      conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback'],
      crossDomainMappings: ['Order-book depth resembles nutrient gradients.'],
      combinations: [
        { name: 'Mycelial order router', description: 'A routing layer that scores venues by local congestion signals. Hypothesis only.' },
        { name: 'Graceful-degradation mesh', description: 'A trade mesh that reroutes through redundant paths when a venue stalls. Hypothesis only.' },
      ],
      feasibility: {
        hypotheses: ['Local congestion signals can rank venues without a central coordinator.'],
        dependencies: ['A venue feed with latency and fill data'],
        unknownCosts: ['Exchange fees under rerouting are unknown.'],
        risks: ['Stale signals could route into worse fills.'],
        testability: 'Replay historical venue data through the router.',
      },
      productization: {
        conceptBrief: 'A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim.',
        architecture: 'Signal ingest, local scoring, policy engine.',
        roadmap: 'Phase 1: simulator.',
        testPlan: 'Replay-test the router.',
        prototypeProposal: 'A paper prototype plus a simulator notebook.',
      },
    },
    provider: 'ollama',
    model: 'llama-x',
    disclosure: {
      provider: 'ollama', model: 'llama-x', endpointHost: '127.0.0.1',
      local: true, routingMode: 'auto', routingReason: 'test',
      messageCount: 3, segmentCount: 4, redactionCount: 0,
      estimatedInputTokens: 100, costClass: 'free', usage: null, attempts: [],
    },
  });
}

test('transfer concurrency: two processes submitting the identical transfer create one concept', async () => {
  test.setTimeout(240000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-transfer-concurrency-e2e-'));
  const children: ChildProcess[] = [];

  async function startApi(pairingDir: string): Promise<{ endpoint: string; token: string }> {
    const child = spawn(
      path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : '')),
      ['--port', '0', '--pairing-root', pairingDir],
      { windowsHide: true, env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') }, stdio: ['ignore', 'pipe', 'ignore'] },
    );
    children.push(child);
    return new Promise((resolve, reject) => {
      let output = '';
      const timer = setTimeout(() => reject(Error('API startup timeout')), 20000);
      child.stdout!.on('data', (data) => {
        output += data.toString();
        const match = output.match(/Pairing file: (.+)\r?\n/);
        if (match) {
          clearTimeout(timer);
          resolve(JSON.parse(fs.readFileSync(match[1].trim(), 'utf8')));
        }
      });
    });
  }

  async function stopAll() {
    for (const child of children) {
      if (child.exitCode === null) {
        const ended = new Promise((resolve) => child.once('exit', resolve));
        child.kill();
        await ended;
      }
    }
  }

  function call(pairing: { endpoint: string; token: string }, group: string, operation: string, input: Record<string, unknown>) {
    return fetch(pairing.endpoint + '/v1/' + group + '/' + operation, {
      method: 'POST',
      headers: { Origin: 'http://127.0.0.1:1420', Authorization: `Bearer ${pairing.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ contractVersion: 1, ...input }),
    }).then(async (response) => ({ status: response.status, body: await response.json() as Record<string, any> }));
  }

  try {
    // The first process creates the workspace and
    // project so the database exists before the
    // second process opens the same root.
    const first = await startApi(path.join(root, 'pairing-1'));
    const created = await call(first, 'workspaces', 'create', { name: 'Transfer Concurrency Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call(first, 'workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
    const project = await call(first, 'memories', 'create', {
      workspaceId,
      kind: 'project',
      draft: {
        title: 'Harbor App', body: 'A test project.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    const projectId = project.body.memory.id as string;
    const registered = await call(first, 'artifacts', 'register', {
      workspaceId, projectId, type: 'experiment', title: 'Collider experiment: Trading signals',
      body: experimentBody(), sourceReferences: [], privacy: 'private',
    });
    const experimentId = registered.body.artifact.artifactId as string;

    const second = await startApi(path.join(root, 'pairing-2'));
    const transferInput = {
      workspaceId, projectId, experimentArtifactId: experimentId, experimentVersion: 1,
      combinationIndex: 0, approvedBrief: 'Mycelial order router, user-edited: score venues by local congestion.',
    };
    // Both submissions in flight at once; the
    // transactional receipt check serializes them.
    const [fromFirst, fromSecond] = await Promise.all([
      call(first, 'ai', 'collider-transfer', transferInput),
      call(second, 'ai', 'collider-transfer', transferInput),
    ]);
    expect(fromFirst.status).toBe(200);
    expect(fromSecond.status).toBe(200);
    expect(fromSecond.body.artifact.artifactId).toBe(fromFirst.body.artifact.artifactId);
    expect(
      fromFirst.body.alreadyTransferred || fromSecond.body.alreadyTransferred,
    ).toBe(true);
    const concepts = (await call(first, 'artifacts', 'list', { workspaceId, projectId })).body.artifacts as Array<Record<string, any>>;
    expect(concepts.filter(a => a.type === 'concept').length).toBe(1);

    // A different brief through the second process is a second transfer.
    const different = await call(second, 'ai', 'collider-transfer', {
      ...transferInput,
      approvedBrief: 'A different approved brief for the same combination.',
    });
    expect(different.status).toBe(200);
    expect(different.body.artifact.artifactId).not.toBe(fromFirst.body.artifact.artifactId);
    const conceptsAfter = (await call(first, 'artifacts', 'list', { workspaceId, projectId })).body.artifacts as Array<Record<string, any>>;
    expect(conceptsAfter.filter(a => a.type === 'concept').length).toBe(2);
  } finally {
    await stopAll();
  }
});
