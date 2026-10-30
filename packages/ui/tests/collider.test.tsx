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
  listAllMemories,
  loadMemoryReviewEntries,
  loadProjectReviewEntries,
  countHiddenProjectItems,
} from '../src/Collider';
import {
  ColliderPipelineOverview,
  colliderResultHeadingId,
  pipelineStateForRun,
} from '../src/ColliderOverview';

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
  assert.match(html, /Saved-project selection is available below/);
  assert.match(html, /URL fetching for either beam/);
  assert.match(html, /Cancelling a fetch drops its outcome/);
  assert.match(html, /bounded total time and saves nothing/);
  assert.match(html, /3D visualization remains deferred/);
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

test('experiment body parser accepts project context and rejects malformed entries', () => {
  const withProject = JSON.parse(validExperimentBody());
  withProject.projectContext = {
    projectId: 'p'.repeat(32),
    projectTitle: 'Harbor notes',
    items: [
      {
        kind: 'memory',
        memoryId: 'm'.repeat(32),
        revisionId: 'r'.repeat(32),
        artifactId: null,
        artifactType: null,
        version: 2,
        title: 'Tide notes',
        content: '  Revision B tide observations.\n',
        contentHash: 'a'.repeat(64),
        privacy: 'private',
      },
      {
        kind: 'artifact',
        memoryId: null,
        revisionId: null,
        artifactId: 'a'.repeat(32),
        artifactType: 'note',
        version: 3,
        title: 'Harbor summary',
        content: 'Summary body.',
        contentHash: 'b'.repeat(64),
        privacy: 'standard',
      },
    ],
  };
  const parsed = parseColliderExperimentBody(JSON.stringify(withProject));
  assert.ok(parsed);
  assert.equal(parsed!.projectContext!.projectTitle, 'Harbor notes');
  assert.equal(parsed!.projectContext!.items.length, 2);
  assert.equal(parsed!.projectContext!.items[0].content, '  Revision B tide observations.\n');
  // A restored artifact item parses with its restored hash preserved,
  // while a memory item carrying one refuses (memory bodies restore
  // verbatim, so the field never belongs on them).
  const restoredDoc = JSON.parse(validExperimentBody());
  restoredDoc.projectContext = {
    projectId: 'p'.repeat(32),
    projectTitle: 'Harbor notes',
    items: [
      { ...withProject.projectContext.items[1], restoredContentHash: 'c'.repeat(64) },
    ],
  };
  const restoredParsed = parseColliderExperimentBody(JSON.stringify(restoredDoc));
  assert.ok(restoredParsed);
  assert.equal(
    restoredParsed!.projectContext!.items[0].restoredContentHash,
    'c'.repeat(64),
  );
  const badMemoryDoc = JSON.parse(validExperimentBody());
  badMemoryDoc.projectContext = {
    projectId: 'p'.repeat(32),
    projectTitle: 'Harbor notes',
    items: [{ ...withProject.projectContext.items[0], restoredContentHash: 'c'.repeat(64) }],
  };
  assert.equal(parseColliderExperimentBody(JSON.stringify(badMemoryDoc)), null);
  // Older experiments without the field still parse, with no project context.
  const legacy = parseColliderExperimentBody(validExperimentBody())!;
  assert.equal(legacy.projectContext, undefined);
  // Malformed entries refuse: sealed privacy, approval artifact type,
  // duplicate item, missing hash, memory item carrying artifact fields.
  const memoryItem = withProject.projectContext.items[0];
  for (const bad of [
    { ...memoryItem, privacy: 'sealed' },
    { ...memoryItem, contentHash: '' },
    { ...memoryItem, artifactId: 'a'.repeat(32) },
    { ...withProject.projectContext.items[1], artifactType: 'transfer-approval' },
    { ...withProject.projectContext.items[1], artifactType: 'specification-approval' },
  ]) {
    const doc = JSON.parse(validExperimentBody());
    doc.projectContext = {
      projectId: 'p'.repeat(32),
      projectTitle: 'Harbor notes',
      items: [bad],
    };
    assert.equal(parseColliderExperimentBody(JSON.stringify(doc)), null);
  }
  const dup = JSON.parse(validExperimentBody());
  dup.projectContext = {
    projectId: 'p'.repeat(32),
    projectTitle: 'Harbor notes',
    items: [memoryItem, memoryItem],
  };
  assert.equal(parseColliderExperimentBody(JSON.stringify(dup)), null);
  const tooMany = JSON.parse(validExperimentBody());
  tooMany.projectContext = {
    projectId: 'p'.repeat(32),
    projectTitle: 'Harbor notes',
    items: Array.from({ length: 9 }, (_, i) => ({
      ...memoryItem,
      memoryId: `m${i}`.padEnd(32, 'x'),
    })),
  };
  assert.equal(parseColliderExperimentBody(JSON.stringify(tooMany)), null);
});

test('experiment view shows generation-time project context verbatim', () => {
  const doc = JSON.parse(validExperimentBody());
  doc.projectContext = {
    projectId: 'p'.repeat(32),
    projectTitle: 'Harbor notes',
    items: [
      {
        kind: 'memory',
        memoryId: 'm'.repeat(32),
        revisionId: 'r'.repeat(32),
        artifactId: null,
        artifactType: null,
        version: 2,
        title: 'Tide notes',
        content: 'Revision B tide observations.',
        contentHash: 'a'.repeat(64),
        privacy: 'private',
      },
      {
        kind: 'artifact',
        memoryId: null,
        revisionId: null,
        artifactId: 'a'.repeat(32),
        artifactType: 'note',
        version: 3,
        title: 'Harbor summary',
        content: 'Summary body.',
        contentHash: 'b'.repeat(64),
        privacy: 'standard',
      },
    ],
  };
  const parsed = parseColliderExperimentBody(JSON.stringify(doc))!;
  const html = renderToStaticMarkup(<ColliderExperimentView doc={parsed} />);
  assert.match(html, /Project context \(as sent at generation time\)/);
  assert.match(html, /From project Harbor notes/);
  assert.match(html, /Tide notes · memory revision 2/);
  assert.match(html, /Revision B tide observations\./);
  assert.match(html, /Harbor summary · artifact version 3/);
  assert.match(html, /Summary body\./);
  assert.match(html, /later project edits do not change it/);
  const legacy = renderToStaticMarkup(
    <ColliderExperimentView doc={parseColliderExperimentBody(validExperimentBody())!} />,
  );
  assert.doesNotMatch(legacy, /Project context \(as sent/);
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

test('picker list helper pages memory lists to exhaustion without silent truncation', async () => {
  const total = 250;
  const rows = Array.from({ length: total }, (_, i) => ({
    id: `m${i}`,
    kind: i === 0 ? 'project' : 'note',
    revisionId: `r${i}`,
    version: 1,
    title: `Item ${i}`,
    privacy: 'private',
  }));
  let calls = 0;
  const client = {
    listMemories: async (_ws: string, options: { offset?: number; limit?: number }) => {
      calls += 1;
      const offset = options.offset ?? 0;
      const limit = options.limit ?? 50;
      const page = rows.slice(offset, offset + limit);
      return {
        memories: page,
        nextOffset: offset + limit < total ? offset + limit : null,
      };
    },
  } as any;
  const all = await listAllMemories(client, 'w'.repeat(32), { kind: 'project' });
  assert.equal(all.length, total);
  assert.equal(all[0].id, 'm0');
  assert.equal(all[249].id, 'm249');
  assert.equal(calls, 3);
  // Exact page boundary: 200 rows take exactly 2 pages, no extra call.
  const rows200 = rows.slice(0, 200);
  let calls200 = 0;
  const client200 = {
    listMemories: async (_ws: string, options: { offset?: number; limit?: number }) => {
      calls200 += 1;
      const offset = options.offset ?? 0;
      const limit = options.limit ?? 50;
      const page = rows200.slice(offset, offset + limit);
      return {
        memories: page,
        nextOffset: offset + limit < 200 ? offset + limit : null,
      };
    },
  } as any;
  const all200 = await listAllMemories(client200, 'w'.repeat(32), {});
  assert.equal(all200.length, 200);
  assert.equal(calls200, 2);
  // Empty list stays empty with a single call.
  const empty = {
    listMemories: async () => ({ memories: [], nextOffset: null }),
  } as any;
  assert.deepEqual(await listAllMemories(empty, 'w'.repeat(32), {}), []);
  // A failure mid-pages rejects: the caller gets an error, never a
  // partial list presented as complete.
  const failing = {
    listMemories: async (_ws: string, options: { offset?: number; limit?: number }) => {
      if ((options.offset ?? 0) > 0) throw new Error('page two exploded');
      return { memories: rows.slice(0, 100), nextOffset: 100 };
    },
  } as any;
  await assert.rejects(listAllMemories(failing, 'w'.repeat(32), {}), /page two exploded/);
  // A page that does not advance rejects instead of looping forever.
  const stuck = {
    listMemories: async () => ({ memories: rows.slice(0, 100), nextOffset: 0 }),
  } as any;
  await assert.rejects(listAllMemories(stuck, 'w'.repeat(32), {}), /did not advance/);
  // Cancellation between pages returns what was loaded without
  // fetching further pages; the effect discards it.
  let cancelled = false;
  let cancelCalls = 0;
  const cancellable = {
    listMemories: async (_ws: string, options: { offset?: number; limit?: number }) => {
      cancelCalls += 1;
      if ((options.offset ?? 0) === 0) cancelled = true;
      const offset = options.offset ?? 0;
      const limit = options.limit ?? 50;
      const page = rows.slice(offset, offset + limit);
      return {
        memories: page,
        nextOffset: offset + limit < total ? offset + limit : null,
      };
    },
  } as any;
  const partial = await listAllMemories(cancellable, 'w'.repeat(32), {}, () => cancelled);
  assert.equal(partial.length, 100);
  assert.equal(cancelCalls, 1);
});

test('picker review loaders capture privacy labels and fail on any unread item', async () => {
  const client = {
    readMemory: async (_ws: string, id: string, revisionId?: string) => ({
      memory: {
        id,
        revisionId: revisionId ?? 'r-current',
        version: revisionId ? 2 : 3,
        title: `Memory ${id}`,
        privacy: id === 'm-sealed-never' ? 'sealed' : 'standard',
      },
      body: `Body ${id}.`,
    }),
    readProjectArtifact: async (_ws: string, artifactId: string, version?: number) => ({
      artifact: { artifactId, version: version ?? 1, title: `Artifact ${artifactId}`, privacy: 'private' },
      body: `Artifact body ${artifactId}.`,
    }),
  } as any;
  const memoryEntries = await loadMemoryReviewEntries(client, 'w'.repeat(32), ['m1', 'm2']);
  assert.equal(memoryEntries.length, 2);
  assert.equal(memoryEntries[0].privacy, 'standard');
  assert.equal(memoryEntries[1].body, 'Body m2.');
  const projectEntries = await loadProjectReviewEntries(client, 'w'.repeat(32), [
    { kind: 'memory', id: 'm1', versionId: 'r2', version: 2, title: 'M1' },
    { kind: 'artifact', id: 'a1', versionId: '3', version: 3, title: 'A1' },
  ]);
  assert.equal(projectEntries.length, 2);
  assert.equal(projectEntries[0].privacy, 'standard');
  assert.equal(projectEntries[0].revisionId, 'r2');
  assert.equal(projectEntries[1].privacy, 'private');
  assert.equal(projectEntries[1].version, 3);
  const failing = {
    ...client,
    readMemory: async (_ws: string, id: string) => {
      if (id === 'm2') throw new Error('unreadable');
      return client.readMemory(_ws, id);
    },
  } as any;
  await assert.rejects(loadMemoryReviewEntries(failing, 'w'.repeat(32), ['m1', 'm2']), /unreadable/);
});

test('picker hidden-item counts cover project records and ineligible artifacts', () => {
  assert.deepEqual(
    countHiddenProjectItems(
      [
        { kind: 'project', id: 'p' },
        { kind: 'note', id: 'n1' },
        { kind: 'note', id: 'n2' },
      ] as any,
      [{ type: 'note' }, { type: 'transfer-approval' }, { type: 'evaluation' }],
    ),
    { hiddenMembers: 1, hiddenArtifacts: 1 },
  );
  assert.deepEqual(countHiddenProjectItems([], []), { hiddenMembers: 0, hiddenArtifacts: 0 });
});

test('pipeline overview maps every genuine run state and never invents progress', () => {
  assert.equal(pipelineStateForRun(null, false), 'not-started');
  assert.equal(pipelineStateForRun({ status: 'running' } as any, false), 'generating');
  assert.equal(pipelineStateForRun({ status: 'complete' } as any, true), 'validating');
  assert.equal(pipelineStateForRun({ status: 'complete' } as any, false), 'complete');
  assert.equal(pipelineStateForRun({ status: 'failed' } as any, false), 'failed');
  assert.equal(pipelineStateForRun({ status: 'invalid' } as any, false), 'invalid');
  assert.equal(pipelineStateForRun({ status: 'cancelling' } as any, false), 'cancelling');
  assert.equal(pipelineStateForRun({ status: 'cancelled' } as any, false), 'cancelled');
  assert.equal(pipelineStateForRun({ status: 'weird' } as any, false), 'failed');
});

test('pipeline overview renders states, links only when complete, and no animation', () => {
  const notStarted = renderToStaticMarkup(<ColliderPipelineOverview state="not-started" />);
  assert.match(notStarted, /Not started/);
  assert.match(notStarted, /Source understanding/);
  assert.match(notStarted, /Proposed combinations/);
  assert.doesNotMatch(notStarted, /href="#collider-/);
  assert.match(notStarted, /<svg/);
  assert.doesNotMatch(notStarted, /<animate|animation/);
  const generating = renderToStaticMarkup(
    <ColliderPipelineOverview state="generating" partialChars={123} />,
  );
  assert.match(generating, /Generating/);
  assert.match(generating, /123 stream characters received/);
  assert.match(generating, /no per-section progress/);
  const complete = renderToStaticMarkup(<ColliderPipelineOverview state="complete" />);
  assert.match(complete, /Complete/);
  for (const id of [
    'collider-understanding-heading',
    'collider-primitives-heading',
    'collider-mappings-heading',
    'collider-combinations-heading',
    'collider-feasibility-heading',
    'collider-productization-heading',
  ]) {
    assert.match(complete, new RegExp(`href="#${id}"`));
  }
  const invalid = renderToStaticMarkup(
    <ColliderPipelineOverview state="invalid" error="combinations: too few" />,
  );
  assert.match(invalid, /did not pass validation/);
  assert.match(invalid, /too few/);
  const saved = renderToStaticMarkup(
    <ColliderPipelineOverview state="complete" hasDisclosure />,
  );
  assert.match(saved, /href="#providers-disclosure-title"/);
  assert.doesNotMatch(saved, /Saved as artifact/);
  const noDisclosure = renderToStaticMarkup(<ColliderPipelineOverview state="complete" />);
  assert.doesNotMatch(noDisclosure, /providers-disclosure-title/);
});

test('saved experiment view shows the complete pipeline overview', () => {
  const doc = parseColliderExperimentBody(validExperimentBody())!;
  const html = renderToStaticMarkup(<ColliderExperimentView doc={doc} />);
  assert.match(html, /Collision pipeline/);
  assert.match(html, /Complete/);
  assert.match(html, /href="#collider-combinations-heading"/);
});

test('R3: saved result headings and overview links share one instance scope', () => {
  assert.equal(colliderResultHeadingId(undefined, 'combinations'), 'collider-combinations-heading');
  const scope = 'saved-abcdef0123456789abcdef0123456789-v1';
  assert.equal(
    colliderResultHeadingId(scope, 'combinations'),
    'collider-saved-abcdef0123456789abcdef0123456789-v1-combinations-heading',
  );
  const doc = parseColliderExperimentBody(validExperimentBody())!;
  const html = renderToStaticMarkup(<ColliderExperimentView doc={doc} instanceScope={scope} />);
  assert.match(html, /href="#collider-saved-abcdef0123456789abcdef0123456789-v1-combinations-heading"/);
  assert.match(html, /id="collider-saved-abcdef0123456789abcdef0123456789-v1-combinations-heading"/);
  assert.match(html, /aria-labelledby="collider-saved-abcdef0123456789abcdef0123456789-v1-combinations-heading"/);
  assert.doesNotMatch(html, /href="#collider-combinations-heading"/);
  const live = renderToStaticMarkup(<ColliderResultView result={validResult()} />);
  assert.match(live, /id="collider-combinations-heading"/);
});

/**
 * M8 document input: strict decode, provenance parsing
 * and race-safe load sessions. Decode tests use Node's
 * WebCrypto (the same globalThis.crypto.subtle the
 * browser path uses); load-session races use deferred
 * promises, never sleeps.
 */
import {
  DocumentLoadSession,
  decodeDocumentBytes,
} from '../src/documentInput';
import { parseColliderDocumentSource, parseColliderUrlSource } from '../src/Collider';
import { UrlFetchSession, urlSourceFromFetch } from '../src/urlFetch';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

test('document decode strips one BOM, preserves CRLF/indent/terminal newline, and hashes both byte views', async () => {
  const bom = new Uint8Array([0xef, 0xbb, 0xbf]);
  const body = new TextEncoder().encode('  First line\r\n\tSecond line\r\n');
  const bytes = new Uint8Array(bom.length + body.length);
  bytes.set(bom, 0);
  bytes.set(body, bom.length);
  const loaded = await decodeDocumentBytes(bytes, 'field-notes.MD');
  assert.equal(loaded.text, '  First line\r\n\tSecond line\r\n');
  assert.equal(loaded.source.label, 'field-notes.MD');
  assert.equal(loaded.source.kind, 'md');
  assert.equal(loaded.source.bom, true);
  assert.equal(loaded.source.rawBytes, bytes.length);
  assert.equal(loaded.source.textChars, [...loaded.text].length);
  assert.match(loaded.source.rawHash, /^[0-9a-f]{64}$/);
  assert.match(loaded.source.textHash, /^[0-9a-f]{64}$/);
  assert.notEqual(loaded.source.rawHash, loaded.source.textHash);
  // The raw hash covers the picked bytes (BOM included):
  // recomputing it here with Node crypto must agree.
  const { createHash } = await import('node:crypto');
  assert.equal(
    loaded.source.rawHash,
    createHash('sha256').update(bytes).digest('hex'),
  );
  assert.equal(
    loaded.source.textHash,
    createHash('sha256').update(new TextEncoder().encode(loaded.text)).digest('hex'),
  );
});

test('document decode refuses invalid UTF-8, empty, whitespace-only, oversized and wrong-kind files', async () => {
  const encode = (s: string) => new TextEncoder().encode(s);
  await assert.rejects(decodeDocumentBytes(new Uint8Array([0xff, 0xfe, 0x61]), 'bad.txt'), /not valid UTF-8/);
  await assert.rejects(decodeDocumentBytes(new Uint8Array(0), 'empty.txt'), /empty/);
  await assert.rejects(decodeDocumentBytes(encode('   \n\t '), 'blank.md'), /empty/);
  await assert.rejects(decodeDocumentBytes(encode('x'.repeat(8001)), 'long.txt'), /at most 8000/);
  await assert.rejects(decodeDocumentBytes(encode('x'), 'notes.pdf'), /Only .txt and .md/);
  await assert.rejects(decodeDocumentBytes(encode('x'), 'notes'), /Only .txt and .md/);
  await assert.rejects(decodeDocumentBytes(new Uint8Array(65537), 'big.txt'), /at most 65536/);
  await assert.rejects(decodeDocumentBytes(encode('x'), 'a/b.txt'), /not a path/);
  // A replacement character in the file is content, not a
  // decode error: strict decoding accepts it verbatim.
  const withReplacement = await decodeDocumentBytes(encode('a\uFFFD b'), 'ok.txt');
  assert.equal(withReplacement.text, 'a\uFFFD b');
});

test('document load session drops stale reads, edits and clears; second selection wins', async () => {
  const session = new DocumentLoadSession();
  const first = deferred<{ text: string }>();
  const firstLoad = session.load(() => first.promise as never);
  const second = deferred<{ text: string }>();
  const secondLoad = session.load(() => second.promise as never);
  first.resolve({ text: 'stale' });
  second.resolve({ text: 'current' });
  assert.equal((await firstLoad).result, null);
  assert.equal((await secondLoad).result?.text, 'current');
  // An edit invalidates a pending read.
  const third = deferred<{ text: string }>();
  const thirdLoad = session.load(() => third.promise as never);
  session.invalidate();
  third.resolve({ text: 'edited-away' });
  assert.equal((await thirdLoad).result, null);
  // A current failure is an outcome, not a throw, so the
  // caller can show it and leave the previous beam
  // untouched.
  const failing = session.load(() => Promise.reject(Error('boom')));
  const failed = await failing;
  assert.equal(failed.result, null);
  assert.equal(failed.error?.message, 'boom');
  assert.equal(failed.generation, session.current);
});

test('document load session gates failures by generation: superseded failures change nothing', async () => {
  const session = new DocumentLoadSession();
  // Superseded by a newer selection: the old failure's
  // generation is stale once the new load is current,
  // and the new load's result still applies.
  const oldFail = deferred<never>();
  const oldLoad = session.load(() => oldFail.promise as never);
  const newOk = deferred<{ text: string }>();
  const newLoad = session.load(() => newOk.promise as never);
  oldFail.reject(Error('obsolete failure'));
  newOk.resolve({ text: 'new chosen content' });
  const oldOutcome = await oldLoad;
  assert.equal(oldOutcome.error?.message, 'obsolete failure');
  assert.notEqual(oldOutcome.generation, session.current);
  assert.equal((await newLoad).result?.text, 'new chosen content');
  // Superseded by an edit: invalidate() is what the
  // panel's edit path calls, so the pending failure is
  // stale and must not surface.
  const editFail = deferred<never>();
  const editLoad = session.load(() => editFail.promise as never);
  session.invalidate();
  editFail.reject(Error('edited-away failure'));
  const editOutcome = await editLoad;
  assert.equal(editOutcome.error?.message, 'edited-away failure');
  assert.notEqual(editOutcome.generation, session.current);
  // Superseded by a clear: the panel's clear path calls
  // the same invalidate(), with the same staleness.
  const clearFail = deferred<never>();
  const clearLoad = session.load(() => clearFail.promise as never);
  session.invalidate();
  clearFail.reject(Error('cleared-away failure'));
  const clearOutcome = await clearLoad;
  assert.equal(clearOutcome.error?.message, 'cleared-away failure');
  assert.notEqual(clearOutcome.generation, session.current);
});

test('document load session: a rejected selection invalidates older pending reads', async () => {
  const session = new DocumentLoadSession();
  // A rejected selection is a bare invalidate with no new
  // load: this is what the panel does before returning
  // from the pre-read size refusal. An obsolete success
  // after it must not apply.
  const oldOk = deferred<{ text: string }>();
  const oldLoad = session.load(() => oldOk.promise as never);
  session.invalidate();
  oldOk.resolve({ text: 'Obsolete content must not win' });
  const oldOutcome = await oldLoad;
  assert.equal(oldOutcome.result, null);
  assert.notEqual(oldOutcome.generation, session.current);
  // The same holds for an obsolete failure: its error is
  // carried but stale, so the caller must not display it
  // over the current rejection.
  const oldFail = deferred<never>();
  const oldFailLoad = session.load(() => oldFail.promise as never);
  session.invalidate();
  oldFail.reject(Error('obsolete failure'));
  const oldFailOutcome = await oldFailLoad;
  assert.equal(oldFailOutcome.error?.message, 'obsolete failure');
  assert.notEqual(oldFailOutcome.generation, session.current);
  // The session is not broken by the bare invalidate: a
  // fresh load afterward still applies.
  const fresh = deferred<{ text: string }>();
  const freshLoad = session.load(() => fresh.promise as never);
  fresh.resolve({ text: 'fresh content' });
  const freshOutcome = await freshLoad;
  assert.equal(freshOutcome.result?.text, 'fresh content');
  assert.equal(freshOutcome.generation, session.current);
});

test('experiment body parser accepts valid document provenance and rejects malformed records', () => {
  const source = {
    label: 'field-notes.md',
    kind: 'md',
    bom: true,
    rawBytes: 29,
    rawHash: 'a'.repeat(64),
    textHash: 'b'.repeat(64),
    textChars: 26,
  };
  assert.deepEqual(parseColliderDocumentSource(source), source);
  assert.equal(parseColliderDocumentSource({ ...source, kind: 'pdf' }), null);
  assert.equal(parseColliderDocumentSource({ ...source, label: 'x/y.md' }), null);
  assert.equal(parseColliderDocumentSource({ ...source, label: 'notes.txt' }), null);
  assert.equal(parseColliderDocumentSource({ ...source, rawHash: 'xyz' }), null);
  assert.equal(parseColliderDocumentSource({ ...source, textChars: 0 }), null);
  assert.equal(parseColliderDocumentSource({ ...source, surprise: 1 }), null);
  const withDoc = JSON.parse(validExperimentBody());
  withDoc.documentA = source;
  const parsed = parseColliderExperimentBody(JSON.stringify(withDoc));
  assert.ok(parsed);
  assert.equal(parsed!.documentA!.label, 'field-notes.md');
  const html = renderToStaticMarkup(<ColliderExperimentView doc={parsed!} />);
  assert.match(html, /Document: field-notes\.md/);
  assert.match(html, /Markdown/);
  assert.match(html, /29 bytes/);
  assert.match(html, /generation-time record/);
  // Documents without the fields still parse as before.
  const plain = parseColliderExperimentBody(validExperimentBody())!;
  assert.equal(plain.documentA, undefined);
  assert.doesNotMatch(
    renderToStaticMarkup(<ColliderExperimentView doc={plain} />),
    /Document:/,
  );
});

test('url fetch session drops stale outcomes, edits, clears and cancels; fresh fetch applies', async () => {
  const session = new UrlFetchSession();
  const first = deferred<any>();
  const firstLoad = session.load(() => first.promise);
  const second = deferred<any>();
  const secondLoad = session.load(() => second.promise);
  first.resolve({ text: 'stale page' });
  second.resolve({ text: 'current page' });
  assert.equal((await firstLoad).result, null);
  assert.equal((await secondLoad).result?.text, 'current page');
  // An edit invalidates a pending fetch.
  const third = deferred<any>();
  const thirdLoad = session.load(() => third.promise);
  session.invalidate();
  third.resolve({ text: 'edited-away' });
  assert.equal((await thirdLoad).result, null);
  // A superseded failure is carried but stale, so the
  // caller must not display it over the current state.
  const oldFail = deferred<never>();
  const oldLoad = session.load(() => oldFail.promise);
  session.invalidate();
  oldFail.reject(Error('obsolete fetch failure'));
  const oldOutcome = await oldLoad;
  assert.equal(oldOutcome.error?.message, 'obsolete fetch failure');
  assert.notEqual(oldOutcome.generation, session.current);
  // Cancel is a bare invalidate: a late success after it
  // must not apply, and a fresh fetch afterward still does.
  const cancelled = deferred<any>();
  const cancelledLoad = session.load(() => cancelled.promise);
  session.invalidate();
  cancelled.resolve({ text: 'cancelled page must not win' });
  assert.equal((await cancelledLoad).result, null);
  const fresh = deferred<any>();
  const freshLoad = session.load(() => fresh.promise);
  fresh.resolve({ text: 'fresh page' });
  const freshOutcome = await freshLoad;
  assert.equal(freshOutcome.result?.text, 'fresh page');
  assert.equal(freshOutcome.generation, session.current);
  // A current failure is an outcome, not a throw.
  const failing = session.load(() => Promise.reject(Error('fetch refused')));
  const failed = await failing;
  assert.equal(failed.result, null);
  assert.equal(failed.error?.message, 'fetch refused');
  assert.equal(failed.generation, session.current);
});

test('url fetch response maps to provenance without its text', () => {
  const source = urlSourceFromFetch({
    contractVersion: 1,
    submittedUrl: 'https://example.com/page',
    finalUrl: 'https://example.com/page',
    contentKind: 'html',
    extraction: 'html-conservative',
    fetchedBytes: 1234,
    fetchedHash: 'a'.repeat(64),
    textHash: 'b'.repeat(64),
    textChars: 42,
    bom: false,
    text: 'Page text.',
  } as any);
  assert.deepEqual(source, {
    submittedUrl: 'https://example.com/page',
    finalUrl: 'https://example.com/page',
    contentKind: 'html',
    extraction: 'html-conservative',
    fetchedBytes: 1234,
    fetchedHash: 'a'.repeat(64),
    textHash: 'b'.repeat(64),
    textChars: 42,
    bom: false,
  });
  assert.equal((source as any).text, undefined);
});

test('experiment body parser accepts valid url provenance and rejects malformed records', () => {
  const source = {
    submittedUrl: 'https://example.com/page',
    finalUrl: 'https://example.com/page',
    contentKind: 'html',
    extraction: 'html-conservative',
    fetchedBytes: 1234,
    fetchedHash: 'a'.repeat(64),
    textHash: 'b'.repeat(64),
    textChars: 42,
    bom: false,
  };
  assert.deepEqual(parseColliderUrlSource(source), source);
  assert.equal(parseColliderUrlSource({ ...source, submittedUrl: 'https://user:pw@example.com/' }), null);
  // A non-default port is not a forgery: the record is
  // inert text and the port is a fetch-policy matter.
  assert.equal(parseColliderUrlSource({ ...source, finalUrl: 'https://example.com:8443/' })?.finalUrl, 'https://example.com:8443/');
  assert.equal(parseColliderUrlSource({ ...source, contentKind: 'pdf' }), null);
  assert.equal(parseColliderUrlSource({ ...source, extraction: 'plain' }), null);
  assert.equal(parseColliderUrlSource({ ...source, fetchedHash: 'xyz' }), null);
  assert.equal(parseColliderUrlSource({ ...source, textChars: 0 }), null);
  assert.equal(parseColliderUrlSource({ ...source, fetchedBytes: 1048577 }), null);
  assert.equal(parseColliderUrlSource({ ...source, surprise: 1 }), null);
  const withUrl = JSON.parse(validExperimentBody());
  withUrl.urlA = source;
  const parsed = parseColliderExperimentBody(JSON.stringify(withUrl));
  assert.ok(parsed);
  assert.equal(parsed!.urlA!.submittedUrl, 'https://example.com/page');
  const html = renderToStaticMarkup(<ColliderExperimentView doc={parsed!} />);
  assert.match(html, /URL: https:\/\/example\.com\/page/);
  assert.match(html, /Web page/);
  assert.match(html, /conservatively extracted text/);
  assert.match(html, /1234 bytes fetched/);
  assert.match(html, /generation-time record/);
  // Experiments without the fields still parse as before.
  const plain = parseColliderExperimentBody(validExperimentBody())!;
  assert.equal(plain.urlA, undefined);
  assert.doesNotMatch(
    renderToStaticMarkup(<ColliderExperimentView doc={plain} />),
    /URL:/,
  );
});
