// M12 Slice 2: Launch Studio generator, bundler, and component rendering.
// Static markup cannot run effects, so click flows (generate, download,
// provider approval) are covered by e2e; what is asserted here is the pure
// logic plus the rendered controls.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import type { ArtifactRecord } from '@anansi/contracts';
import { LaunchStudio } from '../src/LaunchStudio.tsx';
import {
  buildLaunchProviderPrompt,
  buildSomniaNotes,
  buildSwarmtankSummary,
  deriveSimReport,
  estimateTokens,
  generateLaunchCopy,
  launchGate,
  runLaunchProviderDraft,
  supportsLocalFirst,
  type LaunchEvidence,
} from '../src/launchCopy.ts';
import {
  buildLaunchBundle,
  buildLaunchManifest,
  crc32,
} from '../src/launchBundle.ts';

const fullEvidence: LaunchEvidence = {
  conceptName: 'Harbor',
  conceptDescription: 'Small teams lose track of harbor shipments.',
  productizationBrief: 'A board for harbor shipments.',
  specTitle: 'Harbor App',
  specSummary: 'A shipment board for small harbor teams.',
  specPurpose: 'Keep every shipment visible.',
  specIntendedUsers: 'harbor operators',
  specFeatures: ['Live shipment board', 'Offline manifests', 'Local-first data'],
  specPages: ['Board', 'Manifests'],
  specDataAndState: 'Stored locally on device.',
  simInput: {
    description: 'A shipment board.',
    pricing: 'Free tier',
    priceAmount: null,
    segment: 'harbor operators',
    industry: 'shipping',
    features: ['Live shipment board'],
    valueProposition: 'One board for every shipment.',
  },
  simReport: {
    total: 20,
    converts: 12,
    confused: 5,
    rejects: 3,
    conversionRate: 60,
    averageWillingnessToPay: 49,
    topObjections: [{ item: 'Price', count: 4 }],
    topConfusionPoints: [{ item: 'Pricing', count: 2 }],
    objectionBreakdown: [
      { item: 'Price', count: 4 },
      { item: 'Setup', count: 1 },
    ],
    segments: [{ item: 'operator', count: 20 }],
  },
  somnia: {
    scenarios: [
      { id: 'nightmare-missing-evidence-1', instruction: 'Decide.' },
    ],
    results: [{ scenarioId: 'nightmare-missing-evidence-1', decision: 'Refuse' }],
    scenariosTotal: 7,
    completed: 5,
    refused: 1,
    aborted: 1,
    simulatedToolCalls: 12,
    violationsByCategory: [{ category: 'missing-evidence', count: 2 }],
    proposals: [
      {
        id: 'p1',
        action: 'add',
        target: 'evidence checklist',
        rationale: 'Required evidence was absent.',
        proposedChange: 'Add a checklist.',
        evidence: ['nightmare-missing-evidence-1'],
      },
    ],
  },
};

const included = [{ type: 'concept', title: 'Harbor concept', version: 1 }];

function allChannels(evidence: LaunchEvidence): string {
  const copy = generateLaunchCopy(evidence, included);
  return [copy.showHn, copy.productHunt, copy.xThread.join('\n'), copy.releaseNotes].join('\n');
}

test('gate requires a concept and simulation or evaluation evidence', () => {
  assert.deepEqual(launchGate(false, false, false), [
    'a Forge concept artifact',
    'a SwarmTank simulation or SOMNIA evaluation artifact',
  ]);
  assert.deepEqual(launchGate(true, false, false), [
    'a SwarmTank simulation or SOMNIA evaluation artifact',
  ]);
  assert.deepEqual(launchGate(true, true, false), []);
  assert.deepEqual(launchGate(true, false, true), []);
});

test('show HN pitch carries title, problem, solution, and differentiators', () => {
  const copy = generateLaunchCopy(fullEvidence, included);
  assert.ok(copy.showHn.includes('Show HN: Harbor App'));
  assert.ok(copy.showHn.includes('Small teams lose track of harbor shipments.'));
  assert.ok(copy.showHn.includes('A shipment board for small harbor teams.'));
  assert.ok(copy.showHn.includes('- Live shipment board'));
  assert.ok(copy.showHn.includes('Local-first: runs locally'));
});

test('product hunt tagline uses the first feature verbatim', () => {
  const copy = generateLaunchCopy(fullEvidence, included);
  assert.ok(copy.productHunt.includes('Tagline: Harbor App — Live shipment board'));
  assert.ok(copy.productHunt.includes('- Offline manifests'));
  assert.ok(copy.productHunt.includes('One board for every shipment.'));
});

test('x thread has exactly five ordered posts, each within 280 characters', () => {
  const copy = generateLaunchCopy(fullEvidence, included);
  assert.equal(copy.xThread.length, 5);
  copy.xThread.forEach((post, i) => {
    assert.ok(post.startsWith(`${i + 1}/5`), `post ${i + 1} numbered`);
    assert.ok(post.length <= 280, `post ${i + 1} within 280 chars`);
  });
  assert.ok(copy.xThread[1].includes('Small teams lose track'));
  assert.ok(copy.xThread[2].includes('local-first'));
  assert.ok(copy.xThread[4].includes('60%'));
});

test('x thread stays within 280 characters for a very long product name', () => {
  const longName: LaunchEvidence = {
    ...fullEvidence,
    conceptName: 'H'.repeat(300),
    specTitle: undefined,
    specFeatures: ['A very long feature description '.repeat(20)],
  };
  const copy = generateLaunchCopy(longName, included);
  assert.equal(copy.xThread.length, 5);
  for (const post of copy.xThread) assert.ok(post.length <= 280);
});

test('local-first guarantee appears only when the spec supports it', () => {
  assert.ok(supportsLocalFirst(fullEvidence));
  const serverEvidence: LaunchEvidence = {
    ...fullEvidence,
    specFeatures: ['Live shipment board'],
    specDataAndState: 'Server-synced records.',
    specSummary: 'A shipment board.',
  };
  assert.ok(!supportsLocalFirst(serverEvidence));
  const copy = generateLaunchCopy(serverEvidence, included);
  assert.ok(!copy.xThread[2].includes('local-first'));
  assert.ok(copy.xThread[2].includes('dependency-free static prototype'));
});

test('release notes carry deliverables, capabilities, evidence, and prototype status', () => {
  const copy = generateLaunchCopy(fullEvidence, included);
  assert.ok(copy.releaseNotes.includes('concept: Harbor concept (version 1)'));
  assert.ok(copy.releaseNotes.includes('- Live shipment board'));
  assert.ok(copy.releaseNotes.includes('20 personas: 12 Convert, 5 Confused, 3 Reject'));
  assert.ok(copy.releaseNotes.includes('conversion rate 60% (simulated)'));
  assert.ok(copy.releaseNotes.includes('Top objections (simulated):'));
  assert.ok(copy.releaseNotes.includes('7 nightmare scenarios: 5 Complete, 1 Refuse, 1 Abort'));
  assert.ok(
    copy.releaseNotes.includes(
      'Status: Prototype for review; not executed, not deployed.',
    ),
  );
});

test('no invented percentages: only the run rate appears, and only with a run', () => {
  const withSim = allChannels(fullEvidence);
  const percents = [...withSim.matchAll(/(\d+(?:\.\d+)?)%/g)].map(m => m[1]);
  assert.ok(percents.length > 0);
  assert.ok(percents.every(p => p === '60'));
  const { simReport: _drop, simInput: _dropInput, ...noSim } = fullEvidence;
  const withoutSim = allChannels(noSim);
  assert.ok(!withoutSim.includes('%'));
  assert.ok(!withoutSim.includes('testimonial'));
  assert.ok(!withoutSim.includes('revenue'));
});

test('generation is deterministic', () => {
  const a = generateLaunchCopy(fullEvidence, included);
  const b = generateLaunchCopy(fullEvidence, included);
  assert.deepEqual(a, b);
});

test('swarmtank summary lists the full objection matrix, not just the top five', () => {
  const summary = buildSwarmtankSummary(fullEvidence);
  assert.ok(summary);
  assert.ok(summary!.includes('Price: 4'));
  assert.ok(summary!.includes('Setup: 1'));
  assert.ok(summary!.includes('operator: 20'));
});

test('somnia notes label proposals as review-only and never as fixes', () => {
  const notes = buildSomniaNotes(fullEvidence);
  assert.ok(notes);
  assert.ok(notes!.includes('Synthetic replay — not historical events.'));
  assert.ok(notes!.includes('nightmare-missing-evidence-1: Refuse'));
  assert.ok(notes!.includes('Review-only proposals (not applied)'));
  assert.ok(notes!.includes('evidence checklist'));
  assert.ok(notes!.includes('not evidence that anything was fixed'));
  assert.ok(!notes!.includes('has been fixed'));
});

test('crc32 matches the standard check value', () => {
  const ascii = new TextEncoder().encode('123456789');
  assert.equal(crc32(ascii), 0xcbf43926);
});

function listZipEntries(bytes: Uint8Array): string[] {
  const view = new DataView(bytes.buffer);
  const names: string[] = [];
  let offset = 0;
  for (;;) {
    const signature = view.getUint32(offset, true);
    if (signature === 0x02014b50) break;
    assert.equal(signature, 0x04034b50, 'local file header');
    const nameLength = view.getUint16(offset + 26, true);
    const extraLength = view.getUint16(offset + 28, true);
    const size = view.getUint32(offset + 18, true);
    const name = new TextDecoder().decode(
      bytes.slice(offset + 30, offset + 30 + nameLength),
    );
    names.push(name);
    offset += 30 + nameLength + extraLength + size;
  }
  return names;
}

test('bundle is deterministic and entries are sorted by path', () => {
  const encoder = new TextEncoder();
  const files = [
    { path: 'marketing/x-thread.md', content: encoder.encode('thread') },
    { path: 'LAUNCH-MANIFEST.json', content: encoder.encode('{}') },
    { path: 'marketing/show-hn.md', content: encoder.encode('hn') },
  ];
  const a = buildLaunchBundle(files);
  const b = buildLaunchBundle([...files].reverse());
  assert.deepEqual(a, b);
  assert.deepEqual(listZipEntries(a), [
    'LAUNCH-MANIFEST.json',
    'marketing/show-hn.md',
    'marketing/x-thread.md',
  ]);
  assert.equal(a[0], 0x50);
  assert.equal(a[1], 0x4b);
});

test('manifest records exactly the included artifacts', () => {
  const manifest = JSON.parse(
    buildLaunchManifest(
      [
        {
          artifactId: 'a'.repeat(32),
          type: 'concept',
          version: 2,
          contentHashPrefix: '0123456789abcdef',
        },
      ],
      { providerDraft: false, providerLabel: null },
    ),
  );
  assert.equal(manifest.format, 'anansi-launch-bundle');
  assert.equal(manifest.version, 1);
  assert.equal(manifest.generator, 'deterministic-rule-based');
  assert.equal(manifest.providerDraft, false);
  assert.deepEqual(manifest.artifacts, [
    {
      artifactId: 'a'.repeat(32),
      type: 'concept',
      version: 2,
      contentHashPrefix: '0123456789abcdef',
    },
  ]);
});

test('provider prompt contains the evidence and the status line', () => {
  const prompt = buildLaunchProviderPrompt(fullEvidence);
  assert.ok(prompt.includes('Harbor App'));
  assert.ok(prompt.includes('Small teams lose track'));
  assert.ok(prompt.includes('conversion rate 60%'));
  assert.ok(prompt.includes('Prototype for review; not executed, not deployed.'));
  assert.ok(prompt.includes('Do not invent testimonials'));
});

test('provider dispatch seam sends exactly the previewed prompt', async () => {
  const seen: string[] = [];
  const draft = await runLaunchProviderDraft(async prompt => {
    seen.push(prompt);
    return 'provider draft text';
  }, 'the prompt');
  assert.equal(draft, 'provider draft text');
  assert.deepEqual(seen, ['the prompt']);
});

test('token estimate uses ceil(chars / 4)', () => {
  assert.equal(estimateTokens('abcd'), 1);
  assert.equal(estimateTokens('abcde'), 2);
  assert.equal(estimateTokens(''), 0);
});

test('deriveSimReport counts decisions the same way as the panel report', () => {
  const report = deriveSimReport(
    [
      { decision: 'Convert', keyObjection: null, willingnessToPay: 10 },
      { decision: 'Convert', keyObjection: 'Price', willingnessToPay: 20 },
      { decision: 'Confused', keyObjection: 'Price', willingnessToPay: 0 },
      { decision: 'Reject', keyObjection: 'Setup', willingnessToPay: 0 },
    ],
    [{ role: 'operator' }, { role: 'operator' }, { role: 'manager' }, { role: 'operator' }],
  );
  assert.equal(report.total, 4);
  assert.equal(report.converts, 2);
  assert.equal(report.conversionRate, 50);
  assert.equal(report.averageWillingnessToPay, 7.5);
  assert.deepEqual(report.topObjections, [
    { item: 'Price', count: 2 },
    { item: 'Setup', count: 1 },
  ]);
  assert.deepEqual(report.segments, [
    { item: 'operator', count: 3 },
    { item: 'manager', count: 1 },
  ]);
});

function record(
  artifactId: string,
  type: string,
  title: string,
): ArtifactRecord {
  return {
    artifactId,
    projectId: 'p'.repeat(32),
    type,
    version: 1,
    title,
    timestamp: '2026-11-03T00:00:00.000Z',
    sourceReferences: [],
    validationStatus: 'validated',
    privacy: 'private',
    contentHash: '0'.repeat(64),
  };
}

test('studio renders the picker and a disabled Generate with no evidence', () => {
  const html = renderToStaticMarkup(
    <LaunchStudio
      artifacts={[]}
      readArtifact={async () => ''}
    />,
  );
  assert.ok(html.includes('No launch evidence artifacts in this project yet.'));
  assert.ok(html.includes('Generate launch copy'));
  assert.ok(html.includes('disabled'));
});

test('studio lists checked launch artifacts with type and title', () => {
  const html = renderToStaticMarkup(
    <LaunchStudio
      artifacts={[
        record('a'.repeat(32), 'concept', 'Harbor concept'),
        record('b'.repeat(32), 'note', 'Unrelated note'),
      ]}
      readArtifact={async () => ''}
    />,
  );
  assert.ok(html.includes('Include concept: Harbor concept (version 1)'));
  assert.ok(!html.includes('Unrelated note'));
});
