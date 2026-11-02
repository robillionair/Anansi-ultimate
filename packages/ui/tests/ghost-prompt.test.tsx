// M11 GHOST-PROMPT Slice 1: the TS session engine mirrors the Rust
// reference (consent gating, diff spans, pauses, ring/size bounds,
// observable wipe), password scopes are refused, and the strip renders
// OFF/ON states plus the fragments table — with zero innerHTML in the
// new files.

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import {
  GHOST_MAX_EDIT_EVENTS,
  GHOST_MAX_EVENT_TEXT_BYTES,
  GhostSession,
  ghostDiffTexts,
  ghostFnv1aHex,
} from '../src/ghostPrompt.ts';
import { GhostFragmentsTable, GhostPromptStrip, GHOST_CONSENT_TEXT } from '../src/GhostPrompt.tsx';

function enabledSession() {
  const session = GhostSession.create('ghost-test', { id: 'ask-prompt', isPassword: false });
  assert.ok(session);
  session.enable();
  return session;
}

test('ghost session is off by default and records nothing before consent', () => {
  const session = GhostSession.create('ghost-gate', { id: 'ask-prompt', isPassword: false });
  assert.ok(session);
  assert.equal(session.state, 'off');
  assert.equal(session.record('', 'secret draft', null, 0), false);
  assert.equal(session.events.length, 0);
  assert.equal(session.currentText, '');
  session.enable();
  assert.equal(session.consentAcknowledged, true);
  assert.equal(session.record('', 'secret draft', null, 0), true);
});

test('ghost diff spans and offsets match the Rust reference cases', () => {
  const diff = ghostDiffTexts('hello world', 'hello there');
  assert.equal(diff.offset, 6);
  assert.equal(diff.deleted, 'world');
  assert.equal(diff.inserted, 'there');
  assert.deepEqual(ghostDiffTexts('hello', 'hello'), { offset: 5, deleted: '', inserted: '' });
  const session = enabledSession();
  session.record('', 'hello world', 'insert', 0);
  session.record('hello world', 'hello there', 'replace', 500);
  const replaced = session.events[1];
  assert.equal(replaced.kind, 'replace');
  assert.equal(replaced.offset, 6);
  assert.equal(replaced.deleted, 'world');
  session.record('hello there', 'hello', null, 900);
  assert.equal(session.events[2].kind, 'delete');
  assert.equal(session.events[2].deleted, ' there');
  session.record('hello', 'hello pasted', 'paste', 1200);
  assert.equal(session.events[3].kind, 'paste');
  session.record('hello pasted', 'hello', 'undo', 1500);
  assert.equal(session.events[4].kind, 'undo');
  const fragments = session.deletedFragments();
  assert.equal(fragments.length, 3);
  assert.equal(fragments[0].text, 'world');
  assert.equal(fragments[0].hash, ghostFnv1aHex('world'));
});

test('ghost pause detection uses the threshold and records only durations', () => {
  const session = enabledSession();
  session.record('', 'a', null, 0);
  session.record('a', 'ab', null, 1999);
  assert.equal(session.pauses.length, 0);
  session.record('ab', 'abc', null, 2000);
  assert.equal(session.pauses.length, 0);
  session.record('abc', 'abcd', null, 4500);
  assert.equal(session.pauses.length, 1);
  assert.equal(session.pauses[0].durationMs, 2500);
  const rendered = JSON.stringify(session.pauses);
  assert.ok(!/emotion|intent|hesitat|nervous/i.test(rendered));
});

test('ghost resume clears the paused interval', () => {
  const session = enabledSession();
  session.record('', 'a', null, 0);
  session.pause();
  session.resume();
  session.record('a', 'ab', null, 100000);
  assert.equal(session.pauses.length, 0);
});

test('ghost ring buffer and size bound drop oldest first', () => {
  const ring = enabledSession();
  let text = '';
  for (let i = 0; i < 520; i += 1) {
    const next = `${text}x`;
    assert.equal(ring.record(text, next, null, i * 10), true);
    text = next;
  }
  assert.equal(ring.events.length, GHOST_MAX_EDIT_EVENTS);
  assert.equal(ring.events[0].seq, 21);

  const sized = enabledSession();
  const big = 'b'.repeat(20000);
  let current = '';
  for (let i = 0; i < 4; i += 1) {
    const next = `${current}${big}`;
    assert.equal(sized.record(current, next, null, i * 10), true);
    current = next;
  }
  assert.ok(sized.eventTextBytes <= GHOST_MAX_EVENT_TEXT_BYTES);
  assert.equal(sized.events.length, 2);
});

test('ghost password scope is refused and no global capture path exists', () => {
  assert.equal(
    GhostSession.create('ghost-pw', { id: 'login-password', isPassword: true }),
    null,
  );
  const source = readFileSync(new URL('../src/ghostPrompt.ts', import.meta.url), 'utf8');
  assert.ok(!/addEventListener|keydown|keypress|keyup/.test(source));
});

test('ghost wipe is complete and observable', () => {
  const session = enabledSession();
  session.record('', 'alpha requirement', null, 0);
  session.record('alpha requirement', 'beta requirement', null, 100);
  session.record('beta requirement', 'beta', null, 3000);
  assert.equal(session.deletedFragments().length, 2);
  session.stop();
  assert.equal(session.state, 'off');
  assert.equal(session.verifyWiped(), true);
  const serialized = JSON.stringify(session);
  assert.ok(!serialized.includes('alpha'));
  assert.ok(!serialized.includes('beta'));
  assert.ok(!serialized.includes('requirement'));
  session.enable();
  assert.equal(session.events.length, 0);
});

test('ghost consent text is verbatim and the strip renders off and on states', () => {
  assert.equal(
    GHOST_CONSENT_TEXT,
    'When enabled, ANANSI can temporarily track revisions made in this editor. Deleted text remains available for your review until you discard it or explicitly save it.',
  );
  const off = GhostSession.create('ghost-off', { id: 'ask-prompt', isPassword: false });
  assert.ok(off);
  const noop = () => {};
  const offHtml = renderToStaticMarkup(
    <GhostPromptStrip
      engine={off}
      onEnable={noop}
      onPause={noop}
      onResume={noop}
      onClear={noop}
      onStop={noop}
    />,
  );
  assert.ok(offHtml.includes('Enable Draft Exploration'));
  assert.ok(!offHtml.includes('Draft Exploration ON'));

  const on = enabledSession();
  on.record('', 'hello world', 'insert', 0);
  on.record('hello world', 'hello', null, 3000);
  const onHtml = renderToStaticMarkup(
    <GhostPromptStrip
      engine={on}
      onEnable={noop}
      onPause={noop}
      onResume={noop}
      onClear={noop}
      onStop={noop}
    />,
  );
  assert.ok(onHtml.includes('Draft Exploration ON'));
  assert.ok(onHtml.includes('Recording'));
  assert.ok(onHtml.includes('Pause recording'));
  assert.ok(onHtml.includes('Review drafts'));
  assert.ok(onHtml.includes('Clear draft history'));
  assert.ok(onHtml.includes('Stop &amp; discard session'));
});

test('ghost fragments table renders retained text, deleted fragments and pauses', () => {
  const session = enabledSession();
  session.record('', 'red requirement', null, 0);
  session.record('red requirement', 'blue requirement', null, 100);
  session.record('blue requirement', 'blue', null, 3000);
  const cadence = session.cadence();
  const html = renderToStaticMarkup(
    <GhostFragmentsTable
      currentText={session.currentText}
      fragments={session.deletedFragments()}
      pauses={session.pauses}
      insertedChars={cadence.insertedChars}
      editCount={cadence.editCount}
    />,
  );
  assert.ok(html.includes('Retained &amp; Deleted Fragments'));
  assert.ok(html.includes('Retained text: blue'));
  assert.ok(html.includes('<s>red</s>'));
  assert.ok(html.includes('<s> requirement</s>'));
  assert.ok(html.includes('deleted'));
  assert.ok(html.includes('Pause of 2900ms'));
  assert.ok(html.includes('Writing cadence:'));
});

test('ghost new files carry no innerHTML and no provider transmission', () => {
  for (const file of ['../src/ghostPrompt.ts', '../src/GhostPrompt.tsx']) {
    const source = readFileSync(new URL(file, import.meta.url), 'utf8');
    assert.ok(!source.includes('innerHTML'), file);
    assert.ok(!/fetch\(|axios|WebSocket/.test(source), file);
  }
});
