// M12 Slice 1: the pipeline navigator mirrors the Rust engine
// (packages/ui/src/lifecycle.ts vs crates/core/src/pipeline.rs):
// same stage derivation across artifact sets, same Current rule
// (refine never Current, launch reachable), same navigation targets,
// and Launch honestly disabled until Slice 2. Click navigation is
// covered by e2e; static markup cannot run effects.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import { PipelineNavigator } from '../src/PipelineNavigator.tsx';
import { derivePipeline } from '../src/lifecycle.ts';

function markup(types: string[]): string {
  return renderToStaticMarkup(
    <PipelineNavigator artifactTypes={types} onNavigateToAsk={() => {}} />,
  );
}

/** The <li>...</li> segment containing the given stage label. */
function liFor(html: string, label: string): string {
  const at = html.indexOf(`<strong>${label}</strong>`);
  assert.notEqual(at, -1, `stage ${label} rendered`);
  const open = html.lastIndexOf('<li', at);
  const close = html.indexOf('</li>', at);
  return html.slice(open, close);
}

test('fresh project: Discover Current, later stages Awaiting with requirements', () => {
  const html = markup([]);
  assert.ok(html.includes('<nav aria-label="Product Creation Pipeline">'));
  const current = liFor(html, 'Discover');
  assert.ok(current.includes('aria-current="step"'));
  assert.ok(current.includes('Current / In Progress'));
  const build = liFor(html, 'Build');
  assert.ok(build.includes('Awaiting Preceding Stage'));
  assert.ok(build.includes('Needs at least one Forge concept artifact.'));
  assert.ok(build.includes('href="#project-artifacts-heading"'));
  assert.ok(build.includes('Jump to Artifacts (Product Forge)'));
  const launch = liFor(html, 'Launch');
  assert.ok(launch.includes('Awaiting Preceding Stage'));
  assert.ok(launch.includes('Needs a completed simulation or evaluation artifact.'));
  assert.ok(launch.includes('disabled'));
  assert.ok(launch.includes('Launch Studio arrives in Slice 2.'));
});

test('concept plus simulation: Invent and Simulate Complete, Pressure test Current', () => {
  const html = markup(['concept', 'simulation']);
  assert.ok(liFor(html, 'Invent').includes('Complete'));
  assert.ok(liFor(html, 'Simulate').includes('Complete'));
  const pressure = liFor(html, 'Pressure test');
  assert.ok(pressure.includes('aria-current="step"'));
  assert.ok(pressure.includes('Current / In Progress'));
  const launch = liFor(html, 'Launch');
  assert.ok(launch.includes('Available'));
  assert.ok(!launch.includes('Needs a completed simulation'));
  const refine = liFor(html, 'Refine');
  assert.ok(refine.includes('Available'));
  assert.ok(refine.includes('Open Ask ANANSI (GHOST-PROMPT)'));
});

test('pressure test complete: Launch Current, Refine Available and never Current', () => {
  const d = derivePipeline(['concept', 'simulation', 'somnia-evaluation']);
  assert.equal(d.current, 'launch');
  const html = markup(['concept', 'simulation', 'somnia-evaluation']);
  assert.ok(liFor(html, 'Launch').includes('aria-current="step"'));
  const refine = liFor(html, 'Refine');
  assert.ok(refine.includes('Available'));
  assert.ok(!refine.includes('aria-current'));
});

test('full pipeline: pipeline-complete note, no Current stage', () => {
  const html = markup([
    'idea',
    'concept',
    'specification',
    'simulation',
    'somnia-evaluation',
    'launch-material',
  ]);
  assert.ok(html.includes('Pipeline complete: launch materials are registered.'));
  assert.ok(!html.includes('aria-current'));
});

test('evidence counts list only present types with duplicates counted', () => {
  const html = markup(['concept', 'concept', 'simulation']);
  assert.ok(liFor(html, 'Invent').includes('2 concept'));
  assert.ok(liFor(html, 'Simulate').includes('1 simulation'));
  assert.ok(!liFor(html, 'Discover').includes('idea'));
});

test('refine button is disabled without an Ask navigation callback', () => {
  const html = renderToStaticMarkup(<PipelineNavigator artifactTypes={[]} />);
  assert.ok(liFor(html, 'Refine').includes('disabled'));
});
