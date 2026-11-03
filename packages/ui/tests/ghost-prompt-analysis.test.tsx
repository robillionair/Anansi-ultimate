// M11 GHOST-PROMPT Slice 3: the draft-analysis rules mirror the Rust
// engine byte-for-byte (same finding sentences, same evidence), no
// finding text claims intent, approved context includes exactly the
// selected fragments, comparison counts are labeled units, and the
// two-prompt dispatch runner sends exactly the two panel prompts.

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import { GhostSession } from '../src/ghostPrompt.ts';
import {
  analyzeGhostDraft,
  compileApprovedContext,
} from '../src/ghostPrompt.ts';
import {
  ghostCharCount,
  ghostEstimateTokens,
  ghostReferenceResponse,
  runComparisonDispatch,
  GhostPromptCompare,
} from '../src/GhostPromptCompare.tsx';

function sessionWithAllTriggers() {
  const session = GhostSession.create('ghost-analysis', { id: 'ask-prompt', isPassword: false });
  assert.ok(session);
  session.enable();
  session.record('', 'Build a cheap API tool for freelancers with privacy for $50', 'insert', 0);
  session.record(
    'Build a cheap API tool for freelancers with privacy for $50',
    'Build a cheap API tool for freelancers with privacy for $20',
    'replace',
    100,
  );
  session.record(
    'Build a cheap API tool for freelancers with privacy for $20',
    'Build a cheap API tool for freelancers for $20',
    null,
    200,
  );
  session.record(
    'Build a cheap API tool for freelancers for $20',
    'Build a cheap API tool for enterprise for $20',
    null,
    300,
  );
  session.record(
    'Build a cheap API tool for enterprise for $20',
    'Build a cheap simple tool for enterprise for $20',
    null,
    400,
  );
  return session;
}

test('ghost analysis finds all four kinds in order, byte-identical to Rust', () => {
  const session = sessionWithAllTriggers();
  const findings = analyzeGhostDraft(
    session.deletedFragments(),
    session.events,
    session.currentText,
  );
  assert.deepEqual(
    findings.map((finding) => finding.kind),
    ['price-changed', 'privacy-removed', 'audience-shift', 'technical-term-substituted'],
  );
  assert.equal(findings[0].text, 'You changed the requested price several times.');
  assert.equal(findings[1].text, 'You removed a requirement about privacy.');
  assert.equal(findings[2].text, 'You revised the target audience.');
  assert.equal(findings[3].text, 'You replaced a technical term with a broader description.');
  assert.ok(findings[2].evidence.some((item) => item.includes('freelancers') && item.includes('enterprise')));
  assert.ok(findings[3].evidence[0].startsWith('api → '));
});

test('ghost analysis stays silent on negatives and claims no intent', () => {
  const session = GhostSession.create('ghost-analysis-neg', { id: 'ask-prompt', isPassword: false });
  assert.ok(session);
  session.enable();
  session.record('', 'Build a tool with privacy for $20', 'insert', 0);
  session.record('Build a tool with privacy for $20', 'Build a tool with privacy for $20!', null, 100);
  assert.deepEqual(
    analyzeGhostDraft(session.deletedFragments(), session.events, session.currentText),
    [],
  );
  const findings = analyzeGhostDraft(
    sessionWithAllTriggers().deletedFragments(),
    sessionWithAllTriggers().events,
    sessionWithAllTriggers().currentText,
  );
  for (const finding of findings) {
    const lower = finding.text.toLowerCase();
    for (const word of ['subconscious', 'intent', 'feel', 'want', 'think', 'believe', 'mind']) {
      assert.ok(!lower.includes(word), `finding claims intent: ${finding.text}`);
    }
  }
});

test('ghost approved context includes exactly the selected fragments', () => {
  const session = sessionWithAllTriggers();
  const fragments = session.deletedFragments();
  const selected = fragments.filter(
    (fragment) => fragment.text.includes('privacy') || fragment.text.includes('freelancers'),
  );
  assert.equal(selected.length, 2);
  const compiled = compileApprovedContext(session.currentText, selected);
  assert.ok(compiled.startsWith(session.currentText));
  assert.ok(compiled.includes('--- Approved draft context'));
  assert.ok(compiled.includes('with privacy'));
  assert.ok(compiled.includes('freelancers'));
  assert.ok(!compiled.includes('$50'));
  assert.equal(compileApprovedContext(session.currentText, []), session.currentText);
});

test('ghost comparison renders exactly the selected context with labeled counts', () => {
  const session = sessionWithAllTriggers();
  const fragments = session.deletedFragments();
  const selected = fragments.filter((fragment) => fragment.text.includes('privacy'));
  assert.equal(selected.length, 1);
  const html = renderToStaticMarkup(
    <GhostPromptCompare
      finalText={session.currentText}
      selectedFragments={selected}
    />,
  );
  assert.ok(html.includes('Final Prompt Only'));
  assert.ok(html.includes('Final Prompt + Approved Draft Context'));
  assert.ok(html.includes('Exactly what extra context was included:'));
  assert.ok(html.includes('with privacy'));
  assert.ok(!html.includes('freelancers'));
  assert.ok(html.includes('Reference response (offline, not a model answer)'));
  assert.ok(html.includes(`Characters: ${ghostCharCount(session.currentText)}`));
  assert.ok(html.includes(`estimated tokens: ${ghostEstimateTokens(ghostCharCount(session.currentText))}`));
  assert.ok(html.includes('Simulated — assumes $2.00 per 1M tokens'));
  assert.ok(html.includes('Which answer was more useful?'));
  assert.ok(html.includes('Combine into an updated prompt') === false);
  assert.ok(ghostReferenceResponse('hello').startsWith('Reference response (offline, not a model answer)'));
});

test('ghost comparison dispatch runner sends exactly the two panel prompts', async () => {
  const seen: string[] = [];
  const result = await runComparisonDispatch(
    async (prompt) => {
      seen.push(prompt);
      return `answer to ${prompt}`;
    },
    'final prompt',
    'final prompt + context',
  );
  assert.deepEqual(seen, ['final prompt', 'final prompt + context']);
  assert.equal(result.answerA, 'answer to final prompt');
  assert.equal(result.answerB, 'answer to final prompt + context');
});

test('ghost analysis and comparison files carry no innerHTML', () => {
  for (const file of ['../src/GhostPromptCompare.tsx']) {
    const source = readFileSync(new URL(file, import.meta.url), 'utf8');
    assert.ok(!source.includes('innerHTML'), file);
  }
});
