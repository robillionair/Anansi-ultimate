import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import {
  ColliderPanel,
  ColliderResultView,
  ColliderExperimentView,
  parseColliderResult,
  parseColliderExperimentBody,
  DEFAULT_COLLIDER_CONTROLS,
} from '../src/Collider';

/**
 * M8 Slice 1: the collider panel states its bargain and starts
 * nothing before an explicit submission; the client-side parsers
 * are strict mirrors of the backend bounds, so a malformed
 * result or saved body never renders as a partial success.
 */

function validResult() {
  return {
    sourceUnderstanding: 'Two domains with shared coordination patterns.',
    conceptualPrimitives: ['Market clearing', 'Mycelial routing', 'Shared ledger'],
    crossDomainMappings: ['Clearing maps to nutrient routing', 'Ledgers map to hyphal memory'],
    combinations: [
      { name: 'Mycelial market', description: 'Trade routed like nutrients.' },
      { name: 'Ledger hyphae', description: 'Settlement recorded like growth.' },
    ],
    feasibility: {
      hypotheses: ['Routing costs stay bounded in small networks.'],
      dependencies: ['A simulator'],
      unknownCosts: ['Settlement latency at scale'],
      risks: ['Model output may be wrong.'],
      testability: 'Simulate on synthetic trade graphs.',
    },
    productization: {
      conceptBrief: 'A routing simulator for small trade networks.',
      architecture: 'One simulation core, one report view.',
      roadmap: 'Prototype, then synthetic evaluation.',
      testPlan: 'Unit tests for routing, synthetic scenarios.',
      prototypeProposal: 'A single-page simulator sketch.',
    },
  };
}

function validExperimentBody() {
  return JSON.stringify({
    format: 'anansi-collider-experiment',
    version: 1,
    beamA: 'Trading.',
    beamB: 'Mycelium.',
    controls: { ...DEFAULT_COLLIDER_CONTROLS },
    result: validResult(),
    provider: 'ollama',
    model: 'llama-x',
    disclosure: { provider: 'ollama', model: 'llama-x' },
  });
}

test('result parser accepts a valid six-section result and rejects malformed shapes', () => {
  const ok = parseColliderResult(validResult());
  assert.equal(ok?.combinations.length, 2);
  assert.equal(parseColliderResult({ ...validResult(), productization: undefined }), null);
  assert.equal(
    parseColliderResult({ ...validResult(), combinations: [validResult().combinations[0]] }),
    null,
  );
  assert.equal(
    parseColliderResult({ ...validResult(), surprise: 'extra field' }),
    null,
  );
  assert.equal(
    parseColliderResult({
      ...validResult(),
      sourceUnderstanding: 'x'.repeat(6001),
    }),
    null,
  );
  assert.equal(parseColliderResult('not an object'), null);
});

test('experiment body parser preserves beam whitespace verbatim', () => {
  const body = JSON.parse(validExperimentBody());
  body.beamA = "  def route():\n    return 'local'\n";
  body.beamB = "\tMycelium pathways.\n";
  const doc = parseColliderExperimentBody(JSON.stringify(body));
  assert.equal(doc?.beamA, "  def route():\n    return 'local'\n");
  assert.equal(doc?.beamB, "\tMycelium pathways.\n");
});

test('experiment body parser accepts only the collider format with exact controls', () => {
  const doc = parseColliderExperimentBody(validExperimentBody());
  assert.equal(doc?.format, 'anansi-collider-experiment');
  assert.equal(doc?.beamA, 'Trading.');
  assert.equal(parseColliderExperimentBody('not json'), null);
  assert.equal(
    parseColliderExperimentBody(
      JSON.stringify({ ...JSON.parse(validExperimentBody()), format: 'anansi-idea-set' }),
    ),
    null,
  );
  assert.equal(
    parseColliderExperimentBody(
      JSON.stringify({
        ...JSON.parse(validExperimentBody()),
        controls: { ...DEFAULT_COLLIDER_CONTROLS, creativity: 'wild' },
      }),
    ),
    null,
  );
  assert.equal(
    parseColliderExperimentBody(
      JSON.stringify({ ...JSON.parse(validExperimentBody()), beamB: '' }),
    ),
    null,
  );
});

test('panel renders both beams, defaults, deferred scope and starts nothing on render', () => {
  let starts = 0;
  const client = {
    startProjectCollider: () => {
      starts += 1;
      return Promise.resolve({ runId: 'r' });
    },
  } as any;
  const html = renderToStaticMarkup(
    <ColliderPanel
      client={client}
      workspaceId={'a'.repeat(32)}
      projectId={'c'.repeat(32)}
      notify={() => {}}
      onSaved={() => {}}
      busy={false}
    />,
  );
  assert.equal(starts, 0);
  assert.match(html, /Beam A/);
  assert.match(html, /Beam B/);
  assert.match(html, /Collide Ideas/);
  assert.match(html, /Nothing starts until you press Collide Ideas/);
  assert.match(html, /Zero-Cost Lock \(never spend\)/);
  assert.match(html, /Defaults work without configuration/);
  assert.match(html, /do not guarantee novelty/);
  assert.match(html, /URL fetching, document ingestion/);
  assert.match(html, /none of those are available here/);
});

test('result view renders all six sections and never claims validation or novelty', () => {
  const html = renderToStaticMarkup(<ColliderResultView result={validResult() as any} />);
  assert.match(html, /Source understanding/);
  assert.match(html, /Conceptual primitives/);
  assert.match(html, /Cross-domain mappings/);
  assert.match(html, /Proposed combinations or architectures/);
  assert.match(html, /Feasibility hypotheses/);
  assert.match(html, /Productization proposals/);
  assert.match(html, /not researched facts/);
  assert.match(html, /Nothing here establishes novelty/);
  assert.doesNotMatch(html, /validated/);
  assert.doesNotMatch(html, /patentable/);
});

test('experiment view pins provider, controls and exact beams and stays immutable', () => {
  const doc = parseColliderExperimentBody(validExperimentBody())!;
  const html = renderToStaticMarkup(<ColliderExperimentView doc={doc} />);
  assert.match(html, /provider ollama, model llama-x/);
  assert.match(html, /Creativity balanced/);
  assert.match(html, /Beam A \(as supplied\)/);
  assert.match(html, /Trading\./);
  assert.match(html, /this record is immutable/);
  assert.match(html, /not factually verified/);
});

test('experiment body parser accepts memory context and rejects malformed entries', () => {
  const withContext = JSON.parse(validExperimentBody());
  withContext.memoryContext = [
    {
      memoryId: 'm'.repeat(32),
      revisionId: 'r'.repeat(32),
      version: 2,
      title: 'Tide notes',
      content: '  Revision B tide observations.\n',
      contentHash: 'a'.repeat(64),
      privacy: 'private',
    },
  ];
  const parsed = parseColliderExperimentBody(JSON.stringify(withContext));
  assert.ok(parsed);
  assert.equal(parsed!.memoryContext!.length, 1);
  assert.equal(parsed!.memoryContext![0].content, '  Revision B tide observations.\n');
  // Older experiments without the field still parse, with empty context.
  const legacy = parseColliderExperimentBody(validExperimentBody())!;
  assert.deepEqual(legacy.memoryContext, []);
  // Malformed entries refuse: bad privacy, duplicate memory, missing hash.
  for (const bad of [
    { ...withContext.memoryContext[0], privacy: 'sealed' },
    { ...withContext.memoryContext[0], contentHash: '' },
  ]) {
    const doc = JSON.parse(validExperimentBody());
    doc.memoryContext = [bad];
    assert.equal(parseColliderExperimentBody(JSON.stringify(doc)), null);
  }
  const dup = JSON.parse(validExperimentBody());
  dup.memoryContext = [withContext.memoryContext[0], withContext.memoryContext[0]];
  assert.equal(parseColliderExperimentBody(JSON.stringify(dup)), null);
});

test('experiment view shows generation-time memory context verbatim', () => {
  const doc = JSON.parse(validExperimentBody());
  doc.memoryContext = [
    {
      memoryId: 'm'.repeat(32),
      revisionId: 'r'.repeat(32),
      version: 2,
      title: 'Tide notes',
      content: 'Revision B tide observations.',
      contentHash: 'a'.repeat(64),
      privacy: 'private',
    },
  ];
  const parsed = parseColliderExperimentBody(JSON.stringify(doc))!;
  const html = renderToStaticMarkup(<ColliderExperimentView doc={parsed} />);
  assert.match(html, /Memory context \(as sent at generation time\)/);
  assert.match(html, /Tide notes · revision 2/);
  assert.match(html, /Revision B tide observations\./);
  assert.match(html, /later memory edits do not change it/);
  const legacy = renderToStaticMarkup(
    <ColliderExperimentView doc={parseColliderExperimentBody(validExperimentBody())!} />,
  );
  assert.doesNotMatch(legacy, /Memory context \(as sent/);
});

test('panel offers unselected memory context and starts nothing on render', () => {
  let starts = 0;
  const client = {
    startProjectCollider: () => {
      starts += 1;
      return Promise.resolve({ runId: 'r' });
    },
  } as any;
  const html = renderToStaticMarkup(
    <ColliderPanel
      client={client}
      workspaceId={'a'.repeat(32)}
      projectId={'c'.repeat(32)}
      contextMemories={[
        { id: 'm'.repeat(32), title: 'Tide notes', version: 2, privacy: 'private' } as any,
      ]}
      notify={() => {}}
      onSaved={() => {}}
      busy={false}
    />,
  );
  assert.equal(starts, 0);
  assert.match(html, /Memory context \(optional\)/);
  assert.match(html, /Tide notes · Revision 2/);
  assert.match(html, /Nothing is selected automatically/);
});
