// M12 Slice 3: the guided walk derives from the Slice 1 pipeline engine:
// six steps (Define spans Invent + Build), Refine never Complete, and
// Continue targets the first non-Complete step among the five completable
// steps, skipping Refine. Click navigation is covered by e2e; static
// markup cannot run effects.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import { GuidedWalk } from '../src/GuidedWalk.tsx';
import { deriveWalk } from '../src/guidedWalk.ts';

function markup(types: string[]): string {
  return renderToStaticMarkup(
    <GuidedWalk
      artifactTypes={types}
      onNavigateToAsk={() => {}}
      onSelectLaunch={() => {}}
    />,
  );
}

/** The <li>...</li> segment containing the given step label. */
function liFor(html: string, label: string): string {
  const at = html.indexOf(`<strong>${label}</strong>`);
  assert.notEqual(at, -1, `step ${label} rendered`);
  const open = html.lastIndexOf('<li', at);
  const close = html.indexOf('</li>', at);
  return html.slice(open, close);
}

test('fresh project: Discover Current, Define Awaiting, Continue targets Discover', () => {
  const derived = deriveWalk([]);
  assert.equal(derived.continueStep, 'discover');
  assert.equal(derived.complete, false);
  const html = markup([]);
  assert.ok(html.includes('<section aria-labelledby="project-walk-heading">'));
  assert.ok(html.includes('Continue walk'));
  assert.ok(liFor(html, 'Discover').includes('Current / In Progress'));
  const define = liFor(html, 'Define');
  assert.ok(define.includes('Awaiting Preceding Stage'));
  assert.ok(define.includes('Needs at least one Forge concept artifact.'));
  assert.ok(define.includes('href="#project-collider-heading"'));
  const refine = liFor(html, 'Refine');
  assert.ok(refine.includes('Available'));
  assert.ok(refine.includes('Go to Refine'));
});

test('idea only: Discover Complete, Define Current, Define targets artifacts once concept exists', () => {
  const derived = deriveWalk(['idea']);
  assert.equal(derived.continueStep, 'define');
  const html = markup(['idea']);
  assert.ok(liFor(html, 'Discover').includes('Complete'));
  assert.ok(liFor(html, 'Define').includes('Current / In Progress'));
  assert.ok(liFor(html, 'Define').includes('href="#project-collider-heading"'));
  const withConcept = markup(['idea', 'concept']);
  assert.ok(liFor(withConcept, 'Define').includes('Current / In Progress'));
  assert.ok(liFor(withConcept, 'Define').includes('href="#project-artifacts-heading"'));
});

test('concept plus specification: Define Complete, Simulate Current', () => {
  const derived = deriveWalk(['idea', 'concept', 'specification']);
  assert.equal(derived.continueStep, 'simulate');
  const html = markup(['idea', 'concept', 'specification']);
  assert.ok(liFor(html, 'Define').includes('Complete'));
  assert.ok(liFor(html, 'Simulate').includes('Current / In Progress'));
});

test('concept plus simulation without specification: Define Available, not Current', () => {
  const derived = deriveWalk(['idea', 'concept', 'simulation']);
  assert.equal(derived.continueStep, 'define');
  const html = markup(['idea', 'concept', 'simulation']);
  assert.ok(liFor(html, 'Define').includes('Available'));
  assert.ok(liFor(html, 'Simulate').includes('Complete'));
});

test('specification without concept: Define Awaiting, honestly incomplete', () => {
  const derived = deriveWalk(['specification']);
  // Continue still targets the first incomplete completable step: Discover.
  assert.equal(derived.continueStep, 'discover');
  const html = markup(['specification']);
  const define = liFor(html, 'Define');
  assert.ok(define.includes('Awaiting Preceding Stage'));
  assert.ok(define.includes('Needs at least one Forge concept artifact.'));
});

test('full walk except launch-material: Continue targets Launch, Refine still Available', () => {
  const types = ['idea', 'concept', 'specification', 'simulation', 'somnia-evaluation'];
  const derived = deriveWalk(types);
  assert.equal(derived.continueStep, 'launch');
  assert.equal(derived.complete, false);
  const html = markup(types);
  assert.ok(liFor(html, 'Discover').includes('Complete'));
  assert.ok(liFor(html, 'Define').includes('Complete'));
  assert.ok(liFor(html, 'Simulate').includes('Complete'));
  assert.ok(liFor(html, 'Pressure-Test').includes('Complete'));
  assert.ok(liFor(html, 'Refine').includes('Available'));
  assert.ok(!liFor(html, 'Refine').includes('Complete'));
  assert.ok(liFor(html, 'Launch').includes('Current / In Progress'));
});

test('all completable steps done: no Continue, completion note shown, Refine still Available', () => {
  const types = [
    'idea',
    'concept',
    'specification',
    'simulation',
    'somnia-evaluation',
    'launch-material',
  ];
  const derived = deriveWalk(types);
  assert.equal(derived.continueStep, null);
  assert.equal(derived.complete, true);
  const html = markup(types);
  assert.ok(!html.includes('Continue walk'));
  assert.ok(html.includes('Walk complete'));
  assert.ok(html.includes('not a deliverable and never reads Complete'));
  assert.ok(liFor(html, 'Launch').includes('Complete'));
  assert.ok(liFor(html, 'Refine').includes('Available'));
});

test('Refine and Launch buttons render only when their callbacks exist', () => {
  const withoutCallbacks = renderToStaticMarkup(<GuidedWalk artifactTypes={[]} />);
  assert.ok(!withoutCallbacks.includes('Go to Refine'));
  assert.ok(!withoutCallbacks.includes('Go to Launch'));
  // Anchor Continue needs no callback: it still targets Discover.
  assert.ok(withoutCallbacks.includes('Continue walk'));
});
