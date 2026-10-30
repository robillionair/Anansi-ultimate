import { useEffect, useRef, useState } from 'react';
import type {
  Client,
  ColliderControls,
  ColliderDocumentSource,
  ColliderExperimentDoc,
  ColliderUrlSource,
  ColliderMemoryContext,
  ColliderProjectContext,
  ColliderResult,
  DisclosureRecord,
  MemoryMetadata,
} from '@anansi/contracts';
import { DisclosurePanel } from './Providers';
import { ColliderRunController } from './colliderRun';
import {
  DocumentLoadSession,
  MAX_DOCUMENT_BYTES,
  decodeDocumentBytes,
} from './documentInput';
import { UrlFetchSession, urlSourceFromFetch } from './urlFetch';
import {
  ColliderPipelineOverview,
  colliderResultHeadingId,
  pipelineStateForRun,
} from './ColliderOverview';

// ---------------------------------------------------------------------------
// Strict client-side parsing (mirrors the backend bounds in
// crates/core/src/collider.rs). The panel runs this parse
// on a completed run before rendering — that parse is the
// honest "validating" state. A saved experiment body is
// parsed the same way before viewing, so an unrenderable
// saved body shows an error instead of crashing the view
// (the Windows source-review lesson).

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, keys: string[]): boolean {
  const actual = Object.keys(value);
  return (
    actual.length === keys.length && keys.every(key => actual.includes(key))
  );
}

function textField(
  value: unknown,
  max: number,
): string | null {
  if (typeof value !== 'string') return null;
  // Validate on the trimmed length (matching the
  // backend), but return the original text:
  // reopening a saved record must not alter accepted
  // whitespace (indentation, tabs, terminal newlines).
  if (value.trim().length === 0 || value.trim().length > max) return null;
  return value;
}

function textList(
  value: unknown,
  min: number,
  maxItems: number,
  maxChars: number,
): string[] | null {
  if (!Array.isArray(value)) return null;
  if (value.length < min || value.length > maxItems) return null;
  const out: string[] = [];
  for (const item of value) {
    const text = textField(item, maxChars);
    if (text === null) return null;
    out.push(text);
  }
  return out;
}

export function parseColliderResult(value: unknown): ColliderResult | null {
  if (!isRecord(value)) return null;
  if (
    !exactKeys(value, [
      'sourceUnderstanding',
      'conceptualPrimitives',
      'crossDomainMappings',
      'combinations',
      'feasibility',
      'productization',
    ])
  ) {
    return null;
  }
  const sourceUnderstanding = textField(value.sourceUnderstanding, 6000);
  const conceptualPrimitives = textList(value.conceptualPrimitives, 3, 12, 300);
  const crossDomainMappings = textList(value.crossDomainMappings, 2, 12, 600);
  if (
    sourceUnderstanding === null ||
    conceptualPrimitives === null ||
    crossDomainMappings === null
  ) {
    return null;
  }
  if (!Array.isArray(value.combinations)) return null;
  if (value.combinations.length < 2 || value.combinations.length > 8) return null;
  const combinations: { name: string; description: string }[] = [];
  for (const item of value.combinations) {
    if (!isRecord(item) || !exactKeys(item, ['name', 'description'])) return null;
    const name = textField(item.name, 120);
    const description = textField(item.description, 1200);
    if (name === null || description === null) return null;
    combinations.push({ name, description });
  }
  if (
    !isRecord(value.feasibility) ||
    !exactKeys(value.feasibility, [
      'hypotheses',
      'dependencies',
      'unknownCosts',
      'risks',
      'testability',
    ])
  ) {
    return null;
  }
  const hypotheses = textList(value.feasibility.hypotheses, 1, 8, 600);
  const dependencies = textList(value.feasibility.dependencies, 0, 8, 300);
  const unknownCosts = textList(value.feasibility.unknownCosts, 0, 8, 300);
  const risks = textList(value.feasibility.risks, 1, 8, 300);
  const testability = textField(value.feasibility.testability, 1200);
  if (
    hypotheses === null ||
    dependencies === null ||
    unknownCosts === null ||
    risks === null ||
    testability === null
  ) {
    return null;
  }
  if (
    !isRecord(value.productization) ||
    !exactKeys(value.productization, [
      'conceptBrief',
      'architecture',
      'roadmap',
      'testPlan',
      'prototypeProposal',
    ])
  ) {
    return null;
  }
  const conceptBrief = textField(value.productization.conceptBrief, 2000);
  const architecture = textField(value.productization.architecture, 3000);
  const roadmap = textField(value.productization.roadmap, 3000);
  const testPlan = textField(value.productization.testPlan, 3000);
  const prototypeProposal = textField(value.productization.prototypeProposal, 2000);
  if (
    conceptBrief === null ||
    architecture === null ||
    roadmap === null ||
    testPlan === null ||
    prototypeProposal === null
  ) {
    return null;
  }
  return {
    sourceUnderstanding,
    conceptualPrimitives,
    crossDomainMappings,
    combinations,
    feasibility: { hypotheses, dependencies, unknownCosts, risks, testability },
    productization: {
      conceptBrief,
      architecture,
      roadmap,
      testPlan,
      prototypeProposal,
    },
  };
}

const CONTROL_VALUES: Record<keyof Omit<ColliderControls, 'budget'>, string[]> = {
  creativity: ['low', 'balanced', 'high'],
  technicalDepth: ['overview', 'standard', 'deep'],
  practicality: ['speculative', 'balanced', 'practical'],
  innovationDistance: ['near', 'moderate', 'far'],
  preferredOutputType: ['concept', 'architecture', 'product', 'research'],
};

/**
 * Strict parser for saved memory context entries (M8
 * Slice 3): exact keys, bounds mirroring the backend,
 * unique memories, bounded total. Hashes are shape
 * checked only here; the backend re-validates them
 * against revision rows at restore.
 */
export function parseColliderMemoryContext(
  value: unknown,
): ColliderMemoryContext[] | null {
  if (!Array.isArray(value)) return null;
  if (value.length > 8) return null;
  const seen = new Set<string>();
  let total = 0;
  const out: ColliderMemoryContext[] = [];
  for (const item of value) {
    if (!isRecord(item)) return null;
    if (
      !exactKeys(item, [
        'memoryId',
        'revisionId',
        'version',
        'title',
        'content',
        'contentHash',
        'privacy',
      ])
    ) {
      return null;
    }
    if (typeof item.memoryId !== 'string' || item.memoryId.length === 0) return null;
    if (typeof item.revisionId !== 'string' || item.revisionId.length === 0) return null;
    if (seen.has(item.memoryId)) return null;
    seen.add(item.memoryId);
    if (typeof item.version !== 'number' || item.version < 1) return null;
    const title = textField(item.title, 200);
    const content = textField(item.content, 8000);
    if (title === null || content === null) return null;
    total += content.length;
    if (total > 32000) return null;
    if (typeof item.contentHash !== 'string' || item.contentHash.length === 0) return null;
    if (!['private', 'standard', 'public'].includes(item.privacy as string)) return null;
    out.push(item as unknown as ColliderMemoryContext);
  }
  return out;
}

// Mirror of crates/core/src/artifacts.rs ARTIFACT_TYPES.
// The backend refuses anything outside this list at
// dispatch; the project picker only offers these, so
// internal approval rows (transfer-approval,
// specification-approval) are never offered as items.
const ELIGIBLE_PROJECT_ARTIFACT_TYPES = new Set([
  'research',
  'source',
  'idea',
  'concept',
  'experiment',
  'prototype',
  'generated-file',
  'simulation',
  'agent-test',
  'decision',
  'launch-material',
  'feedback',
  'note',
  'rubric',
  'evaluation',
  'deliverable',
  'specification',
  'source-package',
  'source-review',
]);

export function parseColliderProjectContext(
  value: unknown,
): ColliderProjectContext | null {
  if (!isRecord(value)) return null;
  if (!exactKeys(value, ['projectId', 'projectTitle', 'items'])) return null;
  if (typeof value.projectId !== 'string' || value.projectId.length === 0) {
    return null;
  }
  const projectTitle = textField(value.projectTitle, 200);
  if (projectTitle === null) return null;
  if (!Array.isArray(value.items)) return null;
  if (value.items.length < 1 || value.items.length > 8) return null;
  const seen = new Set<string>();
  let total = 0;
  const items: ColliderProjectContext['items'] = [];
  for (const item of value.items) {
    if (!isRecord(item)) return null;
    // An original item carries 11 keys; a restored
    // artifact item additionally carries
    // restoredContentHash, recorded by trusted restore
    // when the referenced artifact's body diverged.
    // Memory items never carry it. Either shape parses;
    // anything else is not a project item.
    const itemKeys = [
      'kind',
      'memoryId',
      'revisionId',
      'artifactId',
      'artifactType',
      'version',
      'title',
      'content',
      'contentHash',
      'privacy',
    ];
    const hasRestored = 'restoredContentHash' in item;
    if (
      !exactKeys(
        item,
        hasRestored ? [...itemKeys, 'restoredContentHash'] : itemKeys,
      )
    ) {
      return null;
    }
    if (
      hasRestored &&
      (typeof item.restoredContentHash !== 'string' ||
        item.restoredContentHash.length === 0)
    ) {
      return null;
    }
    const kind = item.kind as string;
    if (kind !== 'memory' && kind !== 'artifact') return null;
    const key =
      kind === 'memory'
        ? `memory:${item.memoryId as string}`
        : `artifact:${item.artifactId as string}`;
    if (seen.has(key)) return null;
    seen.add(key);
    if (kind === 'memory') {
      if (
        typeof item.memoryId !== 'string' ||
        item.memoryId.length === 0 ||
        typeof item.revisionId !== 'string' ||
        item.revisionId.length === 0 ||
        item.artifactId !== null ||
        item.artifactType !== null ||
        hasRestored
      ) {
        return null;
      }
    } else {
      if (
        typeof item.artifactId !== 'string' ||
        item.artifactId.length === 0 ||
        typeof item.artifactType !== 'string' ||
        !ELIGIBLE_PROJECT_ARTIFACT_TYPES.has(item.artifactType as string) ||
        item.memoryId !== null ||
        item.revisionId !== null
      ) {
        return null;
      }
    }
    if (typeof item.version !== 'number' || item.version < 1) return null;
    const title = textField(item.title, 200);
    const content = textField(item.content, 8000);
    if (title === null || content === null) return null;
    total += content.length;
    if (total > 32000) return null;
    if (typeof item.contentHash !== 'string' || item.contentHash.length === 0) {
      return null;
    }
    if (!['private', 'standard', 'public'].includes(item.privacy as string)) {
      return null;
    }
    items.push(item as unknown as ColliderProjectContext['items'][number]);
  }
  return { projectId: value.projectId, projectTitle, items };
}

/**
 * Strict parse of one beam's document provenance. Mirrors
 * the backend's `validate_document_source` shape checks
 * (hash consistency against the beam is the backend's
 * job at start/restore/transfer time); a malformed record
 * never renders as provenance.
 */
export function parseColliderDocumentSource(
  value: unknown,
): ColliderDocumentSource | null {
  if (!isRecord(value)) return null;
  if (
    !exactKeys(value, [
      'label',
      'kind',
      'bom',
      'rawBytes',
      'rawHash',
      'textHash',
      'textChars',
    ])
  ) {
    return null;
  }
  if (
    typeof value.label !== 'string' ||
    value.label.length === 0 ||
    [...value.label].length > 255 ||
    value.label.includes('/') ||
    value.label.includes('\\') ||
    value.label.includes('\0')
  ) {
    return null;
  }
  const dot = value.label.lastIndexOf('.');
  const suffix =
    dot > 0 ? value.label.slice(dot + 1).toLowerCase() : '';
  if (
    (value.kind !== 'txt' && value.kind !== 'md') ||
    (value.kind === 'txt' && suffix !== 'txt') ||
    (value.kind === 'md' && suffix !== 'md')
  ) {
    return null;
  }
  if (typeof value.bom !== 'boolean') return null;
  if (
    typeof value.rawBytes !== 'number' ||
    !Number.isInteger(value.rawBytes) ||
    value.rawBytes < 0 ||
    value.rawBytes > MAX_DOCUMENT_BYTES
  ) {
    return null;
  }
  const hash = /^[0-9a-f]{64}$/;
  if (
    typeof value.rawHash !== 'string' ||
    !hash.test(value.rawHash) ||
    typeof value.textHash !== 'string' ||
    !hash.test(value.textHash)
  ) {
    return null;
  }
  if (
    typeof value.textChars !== 'number' ||
    !Number.isInteger(value.textChars) ||
    value.textChars < 1 ||
    value.textChars > 8000
  ) {
    return null;
  }
  return value as unknown as ColliderDocumentSource;
}

export function parseColliderUrlSource(
  value: unknown,
): ColliderUrlSource | null {
  if (!isRecord(value)) return null;
  if (!exactKeys(value, [
    'submittedUrl',
    'finalUrl',
    'contentKind',
    'extraction',
    'fetchedBytes',
    'fetchedHash',
    'textHash',
    'textChars',
    'bom',
  ])) {
    return null;
  }
  const hash = /^[0-9a-f]{64}$/;
  for (const key of ['submittedUrl', 'finalUrl'] as const) {
    if (
      typeof value[key] !== 'string' ||
      value[key].length === 0 ||
      [...(value[key] as string)].length > 2048
    ) {
      return null;
    }
    let parsed: URL;
    try {
      parsed = new URL(value[key] as string);
    } catch {
      return null;
    }
    if (
      parsed.protocol !== 'http:' &&
      parsed.protocol !== 'https:'
    ) {
      return null;
    }
    if (parsed.username !== '' || parsed.password !== '') return null;
    if (parsed.hostname === '') return null;
    // No port restriction: see the backend's
    // validate_url_source — the record is inert text.
  }
  if (
    value.contentKind !== 'text' &&
    value.contentKind !== 'html'
  ) {
    return null;
  }
  const expectedExtraction =
    value.contentKind === 'text' ? 'plain' : 'html-conservative';
  if (value.extraction !== expectedExtraction) return null;
  if (
    typeof value.fetchedBytes !== 'number' ||
    !Number.isInteger(value.fetchedBytes) ||
    value.fetchedBytes < 0 ||
    value.fetchedBytes > 1048576
  ) {
    return null;
  }
  if (
    typeof value.fetchedHash !== 'string' ||
    !hash.test(value.fetchedHash) ||
    typeof value.textHash !== 'string' ||
    !hash.test(value.textHash)
  ) {
    return null;
  }
  if (
    typeof value.textChars !== 'number' ||
    !Number.isInteger(value.textChars) ||
    value.textChars < 1 ||
    value.textChars > 8000
  ) {
    return null;
  }
  if (typeof value.bom !== 'boolean') return null;
  return value as unknown as ColliderUrlSource;
}

export function parseColliderExperimentBody(
  body: string,
): ColliderExperimentDoc | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;
  const required = [
    'format',
    'version',
    'beamA',
    'beamB',
    'controls',
    'result',
    'provider',
    'model',
    'disclosure',
  ];
  // memoryContext is optional: experiments saved before
  // Slice 3 carry no context and must still parse.
  // projectContext is likewise optional: experiments
  // saved before Slice 4 carry none and must parse.
  // documentA/documentB are likewise optional: manual
  // beams and older experiments carry none.
  const allowed = [
    ...required,
    'memoryContext',
    'projectContext',
    'documentA',
    'documentB',
    'urlA',
    'urlB',
  ];
  if (
    !required.every(key => Object.keys(parsed).includes(key)) ||
    !Object.keys(parsed).every(key => allowed.includes(key))
  ) {
    return null;
  }
  const memoryContext =
    parsed.memoryContext === undefined
      ? []
      : parseColliderMemoryContext(parsed.memoryContext);
  if (memoryContext === null) return null;
  const projectContext =
    parsed.projectContext === undefined
      ? undefined
      : parseColliderProjectContext(parsed.projectContext);
  if (projectContext === null) return null;
  const documentA =
    parsed.documentA === undefined
      ? undefined
      : parseColliderDocumentSource(parsed.documentA);
  if (documentA === null) return null;
  const documentB =
    parsed.documentB === undefined
      ? undefined
      : parseColliderDocumentSource(parsed.documentB);
  if (documentB === null) return null;
  const urlA =
    parsed.urlA === undefined
      ? undefined
      : parseColliderUrlSource(parsed.urlA);
  if (urlA === null) return null;
  const urlB =
    parsed.urlB === undefined
      ? undefined
      : parseColliderUrlSource(parsed.urlB);
  if (urlB === null) return null;
  if (parsed.format !== 'anansi-collider-experiment' || parsed.version !== 1) {
    return null;
  }
  const beamA = textField(parsed.beamA, 8000);
  const beamB = textField(parsed.beamB, 8000);
  if (beamA === null || beamB === null) return null;
  if (!isRecord(parsed.controls)) return null;
  if (
    !exactKeys(parsed.controls, [
      'creativity',
      'technicalDepth',
      'practicality',
      'innovationDistance',
      'budget',
      'preferredOutputType',
    ])
  ) {
    return null;
  }
  for (const key of Object.keys(CONTROL_VALUES) as (keyof typeof CONTROL_VALUES)[]) {
    if (!CONTROL_VALUES[key].includes(parsed.controls[key] as string)) return null;
  }
  if (
    typeof parsed.controls.budget !== 'string' ||
    parsed.controls.budget.length > 200
  ) {
    return null;
  }
  const result = parseColliderResult(parsed.result);
  if (result === null) return null;
  if (typeof parsed.provider !== 'string' || typeof parsed.model !== 'string') {
    return null;
  }
  if (!isRecord(parsed.disclosure)) return null;
  return {
    ...parsed,
    memoryContext,
    projectContext,
    documentA,
    documentB,
    urlA,
    urlB,
  } as unknown as ColliderExperimentDoc;
}

// ---------------------------------------------------------------------------
// Result rendering

export function ColliderResultView({
  result,
  instanceScope,
}: {
  result: ColliderResult;
  /** Instance scope for heading ids (see colliderResultHeadingId). */
  instanceScope?: string;
}) {
  const heading = (slug: string) => colliderResultHeadingId(instanceScope, slug);
  return (
    <div>
      <p className="help">
        All six sections below are model-generated hypotheses, analogies and
        assumptions — not researched facts. Nothing here establishes novelty,
        patentability, technical validation or market evidence.
      </p>
      <section aria-labelledby={heading('understanding')}>
        <h4 id={heading('understanding')}>Source understanding</h4>
        <p>{result.sourceUnderstanding}</p>
      </section>
      <section aria-labelledby={heading('primitives')}>
        <h4 id={heading('primitives')}>Conceptual primitives</h4>
        <ul>
          {result.conceptualPrimitives.map((primitive, i) => (
            <li key={i}>{primitive}</li>
          ))}
        </ul>
      </section>
      <section aria-labelledby={heading('mappings')}>
        <h4 id={heading('mappings')}>Cross-domain mappings</h4>
        <ul>
          {result.crossDomainMappings.map((mapping, i) => (
            <li key={i}>{mapping}</li>
          ))}
        </ul>
      </section>
      <section aria-labelledby={heading('combinations')}>
        <h4 id={heading('combinations')}>
          Proposed combinations or architectures
        </h4>
        <ul>
          {result.combinations.map((combination, i) => (
            <li key={i}>
              <strong>{combination.name}</strong>: {combination.description}
            </li>
          ))}
        </ul>
      </section>
      <section aria-labelledby={heading('feasibility')}>
        <h4 id={heading('feasibility')}>
          Feasibility hypotheses, dependencies, unknown costs, risks and
          testability
        </h4>
        <p>
          <strong>Hypotheses.</strong>
        </p>
        <ul>
          {result.feasibility.hypotheses.map((item, i) => (
            <li key={i}>{item}</li>
          ))}
        </ul>
        <p>
          <strong>Dependencies.</strong>{' '}
          {result.feasibility.dependencies.length
            ? result.feasibility.dependencies.join('; ')
            : 'None stated.'}
        </p>
        <p>
          <strong>Unknown costs.</strong>{' '}
          {result.feasibility.unknownCosts.length
            ? result.feasibility.unknownCosts.join('; ')
            : 'None stated — costs remain unknown.'}
        </p>
        <p>
          <strong>Risks.</strong>
        </p>
        <ul>
          {result.feasibility.risks.map((item, i) => (
            <li key={i}>{item}</li>
          ))}
        </ul>
        <p>
          <strong>Testability.</strong> {result.feasibility.testability}
        </p>
      </section>
      <section aria-labelledby={heading('productization')}>
        <h4 id={heading('productization')}>Productization proposals</h4>
        <p>
          <strong>Concept brief.</strong> {result.productization.conceptBrief}
        </p>
        <p>
          <strong>Architecture.</strong> {result.productization.architecture}
        </p>
        <p>
          <strong>Roadmap.</strong> {result.productization.roadmap}
        </p>
        <p>
          <strong>Test plan.</strong> {result.productization.testPlan}
        </p>
        <p>
          <strong>Prototype proposal.</strong>{' '}
          {result.productization.prototypeProposal}
        </p>
      </section>
    </div>
  );
}

export function ColliderExperimentView({
  doc,
  instanceScope,
}: {
  doc: ColliderExperimentDoc;
  /** Instance scope for overview links and result headings. */
  instanceScope?: string;
}) {
  return (
    <div>
      <p>
        Collider experiment — provider {doc.provider}, model {doc.model}.
        Beams, controls and result below are exactly as saved; this record is
        immutable. It was structurally validated when generated, not factually
        verified.
      </p>
      <ColliderPipelineOverview state="complete" instanceScope={instanceScope} />
      <p>
        <strong>Controls.</strong> Creativity {doc.controls.creativity} ·
        technical depth {doc.controls.technicalDepth} · practicality{' '}
        {doc.controls.practicality} · innovation distance{' '}
        {doc.controls.innovationDistance} · preferred output{' '}
        {doc.controls.preferredOutputType}
        {doc.controls.budget ? ` · budget: ${doc.controls.budget}` : ''}
      </p>
      <details>
        <summary>Beam A (as supplied)</summary>
        {doc.documentA && (
          <p>
            Document: {doc.documentA.label} ·{' '}
            {doc.documentA.kind === 'md' ? 'Markdown' : 'Plain text'} ·{' '}
            {doc.documentA.rawBytes} bytes · {doc.documentA.textChars}{' '}
            characters · Text SHA-256:{' '}
            <span
              title={`Text SHA-256 ${doc.documentA.textHash}\nFile SHA-256 ${doc.documentA.rawHash}`}
            >
              {doc.documentA.textHash.slice(0, 16)}…
            </span>
            {doc.documentA.bom ? ' · leading BOM stripped' : ''}. This is
            the generation-time record: later changes to the source file
            do not change it.
          </p>
        )}
        {doc.urlA && (
          <p>
            URL: {doc.urlA.submittedUrl}
            {doc.urlA.finalUrl !== doc.urlA.submittedUrl
              ? ` · final URL: ${doc.urlA.finalUrl}`
              : ''}{' '}
            · {doc.urlA.contentKind === 'html' ? 'Web page' : 'Plain text'}
            {doc.urlA.contentKind === 'html'
              ? ' · conservatively extracted text'
              : ''}{' '}
            · {doc.urlA.fetchedBytes} bytes fetched · {doc.urlA.textChars}{' '}
            characters · Text SHA-256:{' '}
            <span
              title={`Submitted URL ${doc.urlA.submittedUrl}\nFinal URL ${doc.urlA.finalUrl}\nText SHA-256 ${doc.urlA.textHash}\nFetched SHA-256 ${doc.urlA.fetchedHash}`}
            >
              {doc.urlA.textHash.slice(0, 16)}…
            </span>
            {doc.urlA.bom ? ' · leading BOM stripped' : ''}. This is
            the generation-time record: later changes at the URL do
            not change it.
          </p>
        )}
        <pre>{doc.beamA}</pre>
      </details>
      <details>
        <summary>Beam B (as supplied)</summary>
        {doc.documentB && (
          <p>
            Document: {doc.documentB.label} ·{' '}
            {doc.documentB.kind === 'md' ? 'Markdown' : 'Plain text'} ·{' '}
            {doc.documentB.rawBytes} bytes · {doc.documentB.textChars}{' '}
            characters · Text SHA-256:{' '}
            <span
              title={`Text SHA-256 ${doc.documentB.textHash}\nFile SHA-256 ${doc.documentB.rawHash}`}
            >
              {doc.documentB.textHash.slice(0, 16)}…
            </span>
            {doc.documentB.bom ? ' · leading BOM stripped' : ''}. This is
            the generation-time record: later changes to the source file
            do not change it.
          </p>
        )}
        {doc.urlB && (
          <p>
            URL: {doc.urlB.submittedUrl}
            {doc.urlB.finalUrl !== doc.urlB.submittedUrl
              ? ` · final URL: ${doc.urlB.finalUrl}`
              : ''}{' '}
            · {doc.urlB.contentKind === 'html' ? 'Web page' : 'Plain text'}
            {doc.urlB.contentKind === 'html'
              ? ' · conservatively extracted text'
              : ''}{' '}
            · {doc.urlB.fetchedBytes} bytes fetched · {doc.urlB.textChars}{' '}
            characters · Text SHA-256:{' '}
            <span
              title={`Submitted URL ${doc.urlB.submittedUrl}\nFinal URL ${doc.urlB.finalUrl}\nText SHA-256 ${doc.urlB.textHash}\nFetched SHA-256 ${doc.urlB.fetchedHash}`}
            >
              {doc.urlB.textHash.slice(0, 16)}…
            </span>
            {doc.urlB.bom ? ' · leading BOM stripped' : ''}. This is
            the generation-time record: later changes at the URL do
            not change it.
          </p>
        )}
        <pre>{doc.beamB}</pre>
      </details>
      {doc.memoryContext && doc.memoryContext.length > 0 && (
        <section aria-label="Memory context">
          <p>
            <strong>Memory context (as sent at generation time).</strong>{' '}
            This is the generation-time snapshot: later memory edits do not
            change it.
          </p>
          {doc.memoryContext.map(entry => (
            <details key={entry.memoryId}>
              <summary>
                {entry.title} · revision {entry.version} · {entry.privacy}
              </summary>
              <pre>{entry.content}</pre>
            </details>
          ))}
        </section>
      )}
      {doc.projectContext && doc.projectContext.items.length > 0 && (
        <section aria-label="Project context">
          <p>
            <strong>
              Project context (as sent at generation time).
            </strong>{' '}
            From project {doc.projectContext.projectTitle}. This is the
            generation-time snapshot: later project edits do not change it.
          </p>
          {doc.projectContext.items.map(entry => (
            <details
              key={
                entry.kind === 'memory'
                  ? `memory:${entry.memoryId}`
                  : `artifact:${entry.artifactId}`
              }
            >
              <summary>
                {entry.title} ·{' '}
                {entry.kind === 'memory' ? 'memory revision' : 'artifact version'}{' '}
                {entry.version} · {entry.privacy}
              </summary>
              <pre>{entry.content}</pre>
            </details>
          ))}
        </section>
      )}
      <ColliderResultView result={doc.result} instanceScope={instanceScope} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Panel

export const DEFAULT_COLLIDER_CONTROLS: ColliderControls = {
  creativity: 'balanced',
  technicalDepth: 'standard',
  practicality: 'balanced',
  innovationDistance: 'moderate',
  budget: '',
  preferredOutputType: 'concept',
};

export type ColliderMemoryReviewEntry = {
  memoryId: string;
  revisionId: string;
  version: number;
  title: string;
  body: string;
  /** Privacy label of the revision that will be sent. */
  privacy: string;
};

// ---------------------------------------------------------------------------
// Picker loading helpers (exported for tests). The component
// wires them to epochs and effect cancellation; the helpers
// themselves are plain async functions over the client, so
// node tests can drive them with deferred fake responses.

export type MemoryListOptions = {
  id?: string;
  kind?: string;
  projectId?: string;
  includeArchived?: boolean;
  includeExpired?: boolean;
};

/** Backend page size for memory lists (limit is capped at 100). */
const MEMORY_LIST_PAGE_LIMIT = 100;
/** Hard page cap: hitting it is a visible error, never a silent truncation. */
const MEMORY_LIST_MAX_PAGES = 200;

/**
 * Follow a memory list to exhaustion, in order. The picker
 * calls list endpoints without a limit by default, which
 * silently truncates at the backend's 50-row page; this
 * helper pages at the 100-row maximum instead. A page
 * that does not advance, or the page cap, is an error.
 * Cancellation is cooperative: the loop checks
 * `isCancelled` between pages and returns what it has,
 * which the caller discards.
 */
export async function listAllMemories(
  client: Client,
  workspaceId: string,
  options: MemoryListOptions,
  isCancelled: () => boolean = () => false,
): Promise<MemoryMetadata[]> {
  const all: MemoryMetadata[] = [];
  let offset = 0;
  for (let page = 0; page < MEMORY_LIST_MAX_PAGES; page += 1) {
    if (isCancelled()) return all;
    const list = await client.listMemories(workspaceId, {
      ...options,
      offset,
      limit: MEMORY_LIST_PAGE_LIMIT,
    });
    all.push(...list.memories);
    if (list.nextOffset === null || list.nextOffset === undefined) return all;
    if (list.nextOffset <= offset) {
      throw new Error('Memory list pagination did not advance.');
    }
    offset = list.nextOffset;
  }
  throw new Error('Memory list is too large to load completely.');
}

/**
 * Read each selected memory's current revision, in order,
 * capturing the privacy label the pre-send review must
 * show. Any failed read rejects the whole review: a
 * partial review must never authorize a send.
 */
export async function loadMemoryReviewEntries(
  client: Client,
  workspaceId: string,
  ids: string[],
): Promise<ColliderMemoryReviewEntry[]> {
  const entries: ColliderMemoryReviewEntry[] = [];
  for (const id of ids) {
    const read = await client.readMemory(workspaceId, id);
    entries.push({
      memoryId: read.memory.id,
      revisionId: read.memory.revisionId,
      version: read.memory.version,
      title: read.memory.title,
      body: read.body,
      privacy: read.memory.privacy,
    });
  }
  return entries;
}

/**
 * Read each chosen project item at its pinned version, in
 * order, capturing the privacy label the pre-send review
 * must show. Any failed read rejects the whole review.
 */
export async function loadProjectReviewEntries(
  client: Client,
  workspaceId: string,
  choices: ColliderProjectChoice[],
): Promise<ColliderProjectReviewEntry[]> {
  const entries: ColliderProjectReviewEntry[] = [];
  for (const choice of choices) {
    if (choice.kind === 'memory') {
      const read = await client.readMemory(workspaceId, choice.id, choice.versionId);
      entries.push({
        kind: 'memory',
        memoryId: read.memory.id,
        revisionId: read.memory.revisionId,
        version: read.memory.version,
        title: read.memory.title,
        body: read.body,
        privacy: read.memory.privacy,
      });
    } else {
      const read = await client.readProjectArtifact(workspaceId, choice.id, choice.version);
      entries.push({
        kind: 'artifact',
        artifactId: read.artifact.artifactId,
        version: read.artifact.version,
        title: read.artifact.title,
        body: read.body,
        privacy: read.artifact.privacy,
      });
    }
  }
  return entries;
}

/**
 * Count list items the picker deliberately hides: project
 * records among a project's members (containers, not
 * content) and ineligible artifact types (approval rows
 * are never offered as context). Counts only — the panel
 * never shows hidden titles or bodies.
 */
export function countHiddenProjectItems(
  members: MemoryMetadata[],
  artifacts: { type: string }[],
): { hiddenMembers: number; hiddenArtifacts: number } {
  return {
    hiddenMembers: members.filter(m => m.kind === 'project').length,
    hiddenArtifacts: artifacts.filter(a => !ELIGIBLE_PROJECT_ARTIFACT_TYPES.has(a.type)).length,
  };
}

export type ColliderProjectChoice = {
  kind: 'memory' | 'artifact';
  /** Memory id or artifact id. */
  id: string;
  /** Revision id for a memory, version number as text for an artifact. */
  versionId: string;
  version: number;
  title: string;
};

export type ColliderProjectReviewEntry = {
  kind: 'memory' | 'artifact';
  memoryId?: string;
  revisionId?: string;
  artifactId?: string;
  version: number;
  title: string;
  body: string;
  /** Privacy label of the pinned revision or version that will be sent. */
  privacy: string;
};

export function ColliderPanel({
  client,
  workspaceId,
  projectId,
  contextMemories = [],
  notify,
  onSaved,
  busy,
}: {
  client: Client;
  workspaceId: string;
  projectId: string;
  /** Workspace memories offered as optional context (nothing selected). */
  contextMemories?: MemoryMetadata[];
  notify: (message: string, isError: boolean) => void;
  onSaved: () => void;
  busy: boolean;
}) {
  const [beamA, setBeamA] = useState('');
  const [beamB, setBeamB] = useState('');
  // Document sources, one per beam. A source describes
  // exactly the beam text it was loaded with; any edit
  // clears it (the backend's hash check would refuse a
  // stale record anyway). A failed read leaves the
  // previous beam and source untouched.
  const [documentA, setDocumentA] =
    useState<ColliderDocumentSource | null>(null);
  const [documentB, setDocumentB] =
    useState<ColliderDocumentSource | null>(null);
  const [documentALoading, setDocumentALoading] = useState(false);
  const [documentBLoading, setDocumentBLoading] = useState(false);
  const [documentAError, setDocumentAError] = useState('');
  const [documentBError, setDocumentBError] = useState('');
  const documentSessionA = useRef(new DocumentLoadSession());
  const documentSessionB = useRef(new DocumentLoadSession());
  // URL sources, one per beam, mirroring document sources:
  // a source describes exactly the beam text it was
  // fetched with; any edit clears it, and a failed fetch
  // leaves the previous beam and source untouched.
  const [urlA, setUrlA] = useState<ColliderUrlSource | null>(null);
  const [urlB, setUrlB] = useState<ColliderUrlSource | null>(null);
  const [urlAInput, setUrlAInput] = useState('');
  const [urlBInput, setUrlBInput] = useState('');
  const [urlALoading, setUrlALoading] = useState(false);
  const [urlBLoading, setUrlBLoading] = useState(false);
  const [urlAError, setUrlAError] = useState('');
  const [urlBError, setUrlBError] = useState('');
  const urlSessionA = useRef(new UrlFetchSession());
  const urlSessionB = useRef(new UrlFetchSession());
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [review, setReview] = useState<ColliderMemoryReviewEntry[] | null>(null);
  const [reviewLoading, setReviewLoading] = useState(false);
  const [reviewError, setReviewError] = useState('');
  // Project context: a saved project plus explicitly
  // chosen items from it. Nothing is selected
  // automatically, and the project's own record text is
  // never included unless the user pastes it as a beam.
  const [projectOptions, setProjectOptions] = useState<MemoryMetadata[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState('');
  const [projectMembers, setProjectMembers] = useState<MemoryMetadata[]>([]);
  const [projectArtifacts, setProjectArtifacts] = useState<
    { artifactId: string; title: string; version: number; type: string }[]
  >([]);
  const [projectChoices, setProjectChoices] = useState<ColliderProjectChoice[]>([]);
  const [projectVersions, setProjectVersions] = useState<
    Record<string, { id: string; version: number }[]>
  >({});
  const [projectReview, setProjectReview] = useState<
    ColliderProjectReviewEntry[] | null
  >(null);
  const [projectReviewLoading, setProjectReviewLoading] = useState(false);
  const [projectReviewError, setProjectReviewError] = useState('');
  // List failures are their own state, never an empty
  // list: an empty list means "nothing to offer", an
  // error means "could not load". Retry counters reload
  // the same lists and reviews without changing the
  // selection.
  const [projectOptionsError, setProjectOptionsError] = useState('');
  const [projectMembersError, setProjectMembersError] = useState('');
  const [projectArtifactsError, setProjectArtifactsError] = useState('');
  const [projectListsLoaded, setProjectListsLoaded] = useState(false);
  const [projectListRetry, setProjectListRetry] = useState(0);
  const [memoryReviewRetry, setMemoryReviewRetry] = useState(0);
  const [projectReviewRetry, setProjectReviewRetry] = useState(0);
  const [projectHiddenMembers, setProjectHiddenMembers] = useState(0);
  const [projectHiddenArtifacts, setProjectHiddenArtifacts] = useState(0);
  const [starting, setStarting] = useState(false);
  const startingRef = useRef(false);
  const mountedRef = useRef(true);
  const reviewReady = selectedIds.length === 0 || (
    !reviewLoading && !reviewError && review !== null &&
    review.length === selectedIds.length &&
    review.every((entry, index) => entry.memoryId === selectedIds[index])
  );
  // The project review must describe exactly the
  // current choices, in order, at the pinned versions:
  // a stale review can never authorize a different set.
  const projectReviewReady = projectChoices.length === 0 || (
    !projectReviewLoading && !projectReviewError && projectReview !== null &&
    projectReview.length === projectChoices.length &&
    projectReview.every((entry, index) => {
      const choice = projectChoices[index];
      if (entry.kind !== choice.kind) return false;
      if (choice.kind === 'memory') {
        return entry.memoryId === choice.id && entry.revisionId === choice.versionId;
      }
      return entry.artifactId === choice.id && String(entry.version) === choice.versionId;
    })
  );
  // Bounds mirror the backend exactly (8 items, 8000 chars
  // per item, 32000 chars total, per picker). A review
  // that violates them blocks submission with the bound
  // named, instead of failing at the backend after the
  // user pressed Collide Ideas. Nothing is truncated.
  const reviewBoundsOk =
    review === null ||
    (review.every(entry => entry.body.length <= 8000) &&
      review.reduce((total, entry) => total + entry.body.length, 0) <= 32000);
  const projectReviewBoundsOk =
    projectReview === null ||
    (projectReview.every(entry => entry.body.length <= 8000) &&
      projectReview.reduce((total, entry) => total + entry.body.length, 0) <= 32000);
  const [controls, setControls] = useState<ColliderControls>(
    DEFAULT_COLLIDER_CONTROLS,
  );
  const [lock, setLock] = useState(true);
  const [run, setRun] = useState<{
    runId: string;
    status: string;
    partialChars: number;
    error: string | null;
  } | null>(null);
  const [validating, setValidating] = useState(false);
  const [result, setResult] = useState<ColliderResult | null>(null);
  const [disclosure, setDisclosure] = useState<DisclosureRecord | null>(null);
  const [savedArtifact, setSavedArtifact] = useState('');

  const controllerRef = useRef<ColliderRunController | null>(null);
  function controller(): ColliderRunController {
    if (!controllerRef.current) {
      controllerRef.current = new ColliderRunController(client, workspaceId);
    }
    return controllerRef.current;
  }

  // Pre-send review: read each selected memory's current
  // revision and show exactly what will be sent. A
  // selection epoch drops late responses for a
  // superseded selection, so a stale review can never
  // describe a different selection than the one shown.
  const reviewEpoch = useRef(0);
  useEffect(() => {
    const epoch = ++reviewEpoch.current;
    setReview(null);
    setReviewError('');
    if (selectedIds.length === 0) {
      setReviewLoading(false);
      return;
    }
    setReviewLoading(true);
    void loadMemoryReviewEntries(client, workspaceId, selectedIds)
      .then(entries => {
        if (epoch !== reviewEpoch.current) return;
        setReview(entries);
        setReviewLoading(false);
      })
      .catch(() => {
        if (epoch !== reviewEpoch.current) return;
        setReview(null);
        setReviewError(
          'A selected memory could not be read; retry below or refresh the selection.',
        );
        setReviewLoading(false);
      });
    return () => {
      if (reviewEpoch.current === epoch) reviewEpoch.current += 1;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedIds.join(','), memoryReviewRetry]);

  // Project context loading and review. Two separate
  // disciplines: option lists are render-only data
  // guarded by effect cancellation (a stale list can
  // never authorize a send), while the review is the
  // consent boundary and uses the same epoch discipline
  // as the memory review. A project switch clears
  // choices, review and version options synchronously,
  // so a late response for the previous project can
  // never describe the new selection.
  const projectReviewEpoch = useRef(0);
  const projectListEpoch = useRef(0);
  useEffect(() => {
    let cancelled = false;
    setProjectOptionsError('');
    void listAllMemories(
      client,
      workspaceId,
      { kind: 'project' },
      () => cancelled,
    )
      .then(list => {
        if (!cancelled) setProjectOptions(list);
      })
      .catch(() => {
        if (!cancelled) {
          setProjectOptions([]);
          setProjectOptionsError(
            'Saved projects could not be listed; retry below.',
          );
        }
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceId, projectListRetry]);
  useEffect(() => {
    if (!selectedProjectId) {
      setProjectMembers([]);
      setProjectArtifacts([]);
      setProjectMembersError('');
      setProjectArtifactsError('');
      setProjectHiddenMembers(0);
      setProjectHiddenArtifacts(0);
      setProjectListsLoaded(true);
      return;
    }
    let cancelled = false;
    setProjectListsLoaded(false);
    setProjectMembersError('');
    setProjectArtifactsError('');
    void (async () => {
      try {
        const members = await listAllMemories(
          client,
          workspaceId,
          { projectId: selectedProjectId },
          () => cancelled,
        );
        if (cancelled) return;
        const hidden = countHiddenProjectItems(members, []);
        setProjectHiddenMembers(hidden.hiddenMembers);
        setProjectMembers(members.filter(m => m.kind !== 'project'));
      } catch {
        if (!cancelled) {
          setProjectMembers([]);
          setProjectMembersError(
            'This project\u2019s members could not be listed; retry below.',
          );
        }
      }
      if (cancelled) return;
      try {
        const list = await client.listProjectArtifacts(workspaceId, selectedProjectId);
        if (cancelled) return;
        const hidden = countHiddenProjectItems([], list.artifacts);
        setProjectHiddenArtifacts(hidden.hiddenArtifacts);
        setProjectArtifacts(
          list.artifacts
            .filter(a => ELIGIBLE_PROJECT_ARTIFACT_TYPES.has(a.type))
            .map(a => ({
              artifactId: a.artifactId,
              title: a.title,
              version: a.version,
              type: a.type,
            })),
        );
      } catch {
        if (!cancelled) {
          setProjectArtifacts([]);
          setProjectArtifactsError(
            'This project\u2019s artifacts could not be listed; retry below.',
          );
        }
      }
      if (!cancelled) setProjectListsLoaded(true);
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedProjectId, projectListRetry]);
  const projectChoicesKey = projectChoices
    .map(c => `${c.kind}:${c.id}@${c.versionId}`)
    .join(',');
  useEffect(() => {
    const epoch = ++projectReviewEpoch.current;
    setProjectReview(null);
    setProjectReviewError('');
    if (projectChoices.length === 0) {
      setProjectReviewLoading(false);
      return;
    }
    setProjectReviewLoading(true);
    void loadProjectReviewEntries(client, workspaceId, projectChoices)
      .then(entries => {
        if (epoch !== projectReviewEpoch.current) return;
        setProjectReview(entries);
        setProjectReviewLoading(false);
      })
      .catch(() => {
        if (epoch !== projectReviewEpoch.current) return;
        setProjectReview(null);
        setProjectReviewError(
          'A selected project item could not be read; retry below or refresh the selection.',
        );
        setProjectReviewLoading(false);
      });
    return () => {
      if (projectReviewEpoch.current === epoch) projectReviewEpoch.current += 1;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectChoicesKey, projectReviewRetry]);

  function changeSelectedProject(id: string) {
    // Invalidate in the input event, before any old
    // read can settle or an immediate submit can reuse
    // the previous review.
    projectReviewEpoch.current += 1;
    projectListEpoch.current += 1;
    setSelectedProjectId(id);
    setProjectChoices([]);
    setProjectReview(null);
    setProjectReviewError('');
    setProjectVersions({});
    setProjectMembers([]);
    setProjectArtifacts([]);
  }
  function toggleProjectMemory(m: MemoryMetadata, checked: boolean) {
    projectReviewEpoch.current += 1;
    setProjectReview(null);
    setProjectReviewError('');
    if (!checked) {
      setProjectChoices(previous =>
        previous.filter(c => !(c.kind === 'memory' && c.id === m.id)),
      );
      return;
    }
    setProjectChoices(previous => [
      ...previous,
      {
        kind: 'memory',
        id: m.id,
        versionId: m.revisionId,
        version: m.version,
        title: m.title,
      },
    ]);
    const epoch = projectListEpoch.current;
    void listAllMemories(client, workspaceId, {
      id: m.id,
      projectId: selectedProjectId,
    })
      .then(list => {
        if (epoch !== projectListEpoch.current) return;
        setProjectVersions(previous => ({
          ...previous,
          [`memory:${m.id}`]: list.map(r => ({
            id: r.revisionId,
            version: r.version,
          })),
        }));
      })
      .catch(() => undefined);
  }
  function toggleProjectArtifact(
    a: { artifactId: string; title: string; version: number; type: string },
    checked: boolean,
  ) {
    projectReviewEpoch.current += 1;
    setProjectReview(null);
    setProjectReviewError('');
    if (!checked) {
      setProjectChoices(previous =>
        previous.filter(c => !(c.kind === 'artifact' && c.id === a.artifactId)),
      );
      return;
    }
    setProjectChoices(previous => [
      ...previous,
      {
        kind: 'artifact',
        id: a.artifactId,
        versionId: String(a.version),
        version: a.version,
        title: a.title,
      },
    ]);
    const epoch = projectListEpoch.current;
    void client
      .listProjectArtifactVersions(workspaceId, a.artifactId)
      .then(list => {
        if (epoch !== projectListEpoch.current) return;
        setProjectVersions(previous => ({
          ...previous,
          [`artifact:${a.artifactId}`]: list.versions.map(v => ({
            id: String(v.version),
            version: v.version,
          })),
        }));
      })
      .catch(() => undefined);
  }
  function changeProjectChoiceVersion(
    choice: ColliderProjectChoice,
    versionId: string,
    version: number,
  ) {
    projectReviewEpoch.current += 1;
    setProjectReview(null);
    setProjectReviewError('');
    setProjectChoices(previous =>
      previous.map(c =>
        c.kind === choice.kind && c.id === choice.id
          ? { ...c, versionId, version }
          : c,
      ),
    );
  }

  // Poll the active run; unmount (project switch, panel
  // close) cancels it through the controller reset.
  useEffect(() => {
    if (!run || (run.status !== 'running' && run.status !== 'cancelling')) {
      return;
    }
    const timer = setInterval(() => {
      void (async () => {
        try {
          const polled = await controller().pollOnce();
          if (!polled) return;
          setRun({
            runId: polled.runId,
            status: polled.status,
            partialChars: polled.partialChars,
            error: polled.error ?? null,
          });
          if (polled.disclosure) setDisclosure(polled.disclosure);
          if (polled.status === 'complete' && polled.result) {
            // Honest validating state: strictly re-parse
            // the completed output before rendering it.
            setValidating(true);
            const parsed = parseColliderResult(
              polled.result as unknown as Record<string, unknown>,
            );
            setValidating(false);
            if (parsed) {
              setResult(parsed);
            } else {
              setRun(r =>
                r
                  ? {
                      ...r,
                      status: 'invalid',
                      error: 'The completed output did not pass validation and was not rendered.',
                    }
                  : r,
              );
            }
          }
        } catch {
          // Transient poll failures keep the run state;
          // the next interval retries.
        }
      })();
    }, 800);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [run?.runId, run?.status]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      reviewEpoch.current += 1;
      void controllerRef.current?.reset();
    };
  }, []);

  function editBeam(
    beam: 'a' | 'b',
    value: string,
  ) {
    // Editing invalidates any pending file read or URL
    // fetch for this beam and drops both attributions:
    // the beam is manual text from here on, and a stale
    // load can never overwrite the edit.
    const documentSession =
      beam === 'a' ? documentSessionA.current : documentSessionB.current;
    const urlSession =
      beam === 'a' ? urlSessionA.current : urlSessionB.current;
    documentSession.invalidate();
    urlSession.invalidate();
    if (beam === 'a') {
      setDocumentALoading(false);
      setDocumentA(null);
      setUrlALoading(false);
      setUrlA(null);
      setBeamA(value);
    } else {
      setDocumentBLoading(false);
      setDocumentB(null);
      setUrlBLoading(false);
      setUrlB(null);
      setBeamB(value);
    }
  }

  function clearDocument(beam: 'a' | 'b') {
    // Clearing a source empties the beam, so both
    // attributions go: the beam holds no sourced text.
    const documentSession =
      beam === 'a' ? documentSessionA.current : documentSessionB.current;
    const urlSession =
      beam === 'a' ? urlSessionA.current : urlSessionB.current;
    documentSession.invalidate();
    urlSession.invalidate();
    if (beam === 'a') {
      setDocumentALoading(false);
      setDocumentA(null);
      setDocumentAError('');
      setUrlALoading(false);
      setUrlA(null);
      setUrlAError('');
      setBeamA('');
    } else {
      setDocumentBLoading(false);
      setDocumentB(null);
      setDocumentBError('');
      setUrlBLoading(false);
      setUrlB(null);
      setUrlBError('');
      setBeamB('');
    }
  }

  function clearUrlSource(beam: 'a' | 'b') {
    // Same as clearing a document source: the beam is
    // emptied, so both attributions go with it.
    clearDocument(beam);
  }

  async function pickDocument(beam: 'a' | 'b', file: File) {
    const session =
      beam === 'a' ? documentSessionA.current : documentSessionB.current;
    const setLoading =
      beam === 'a' ? setDocumentALoading : setDocumentBLoading;
    const setError = beam === 'a' ? setDocumentAError : setDocumentBError;
    const setSource = beam === 'a' ? setDocumentA : setDocumentB;
    const setBeam = beam === 'a' ? setBeamA : setBeamB;
    // Every new selection supersedes older pending reads,
    // including a selection that is itself rejected before
    // reading: invalidate first, before any early return,
    // so a gated read can never land after newer intent.
    // A rejection touches only loading state and error —
    // never the last applied beam or attribution.
    session.invalidate();
    // A document pick is a source switch: it retires a
    // pending URL fetch for this beam as well.
    const urlSession =
      beam === 'a' ? urlSessionA.current : urlSessionB.current;
    const setUrlLoading =
      beam === 'a' ? setUrlALoading : setUrlBLoading;
    urlSession.invalidate();
    setUrlLoading(false);
    // Bound the read before allocating it: a file larger
    // than the decoder's limit is refused on File.size
    // alone and its contents are never read. The decoder
    // keeps its own post-read length check for a file
    // whose size changes between this check and the read.
    if (file.size > MAX_DOCUMENT_BYTES) {
      if (mountedRef.current) {
        // The invalidated read resolves silently without
        // clearing the flag itself, so clear it here.
        setLoading(false);
        setError(`The file must be at most ${MAX_DOCUMENT_BYTES} bytes.`);
      }
      return;
    }
    setLoading(true);
    setError('');
    const loaded = await session.load(async () =>
      decodeDocumentBytes(
        new Uint8Array(await file.arrayBuffer()),
        file.name,
      ),
    );
    if (!mountedRef.current) return;
    // One generation gate for both paths: a superseded
    // load — successful or failed — changes neither the
    // error nor the loading flag, because the invalidating
    // action (or the newer load) already owns them.
    if (loaded.generation !== session.current) return;
    setLoading(false);
    if (loaded.error) {
      // A failed read changes nothing else: the previous
      // beam and attribution stay, and the error is
      // visible with retry (pick the file again).
      setError(loaded.error.message);
      return;
    }
    if (loaded.result) {
      setBeam(loaded.result.text);
      setSource(loaded.result.source);
      // Source switch: a document beam carries no URL
      // record, and a stale URL error no longer applies.
      if (beam === 'a') {
        setUrlA(null);
        setUrlAError('');
      } else {
        setUrlB(null);
        setUrlBError('');
      }
    }
  }

  async function fetchUrl(beam: 'a' | 'b') {
    const session =
      beam === 'a' ? urlSessionA.current : urlSessionB.current;
    const setLoading = beam === 'a' ? setUrlALoading : setUrlBLoading;
    const setError = beam === 'a' ? setUrlAError : setUrlBError;
    const setSource = beam === 'a' ? setUrlA : setUrlB;
    const setBeam = beam === 'a' ? setBeamA : setBeamB;
    const entered = (beam === 'a' ? urlAInput : urlBInput).trim();
    // Every new fetch supersedes older pending loads of
    // either kind for this beam, including a fetch that
    // is itself refused before any network call: retire
    // first, before any early return, so a gated fetch
    // can never land after newer intent. A rejection
    // touches only loading state and error — never the
    // last applied beam or attribution.
    session.invalidate();
    const documentSession =
      beam === 'a' ? documentSessionA.current : documentSessionB.current;
    const setDocumentLoading =
      beam === 'a' ? setDocumentALoading : setDocumentBLoading;
    documentSession.invalidate();
    setDocumentLoading(false);
    if (!entered) {
      if (mountedRef.current) {
        setLoading(false);
        setError('Enter a URL to fetch.');
      }
      return;
    }
    setLoading(true);
    setError('');
    const outcome = await session.load(() =>
      client.fetchProjectColliderUrl(workspaceId, projectId, entered),
    );
    if (!mountedRef.current) return;
    // One generation gate for both paths: a superseded
    // fetch — successful or failed — changes neither the
    // error nor the loading flag, because the invalidating
    // action (or the newer fetch) already owns them.
    if (outcome.generation !== session.current) return;
    setLoading(false);
    if (outcome.error) {
      // A failed fetch changes nothing else: the previous
      // beam and attribution stay, and the error is
      // visible with retry (press Fetch again).
      setError(outcome.error.message);
      return;
    }
    if (outcome.result) {
      setBeam(outcome.result.text);
      setSource(urlSourceFromFetch(outcome.result));
      // Source switch: a fetched beam carries no document
      // record, and a stale document error no longer applies.
      if (beam === 'a') {
        setDocumentA(null);
        setDocumentAError('');
      } else {
        setDocumentB(null);
        setDocumentBError('');
      }
    }
  }

  function cancelUrlFetch(beam: 'a' | 'b') {
    // Cancel drops the outcome, never the beam: the
    // in-flight backend fetch runs to its bounded end but
    // saves nothing, and its outcome is dropped on arrival
    // because the session is no longer current.
    const session =
      beam === 'a' ? urlSessionA.current : urlSessionB.current;
    session.invalidate();
    if (beam === 'a') {
      setUrlALoading(false);
    } else {
      setUrlBLoading(false);
    }
  }

  async function collide() {
    if (!mountedRef.current || startingRef.current || busy || running || documentALoading || documentBLoading || urlALoading || urlBLoading || !reviewReady || !projectReviewReady || !reviewBoundsOk || !projectReviewBoundsOk) return;
    // The immutable revisions already displayed are the consent boundary.
    // Re-reading current heads here could transmit an unseen newer revision.
    // The same holds for project items: only the reviewed refs are sent.
    const memoryContext = selectedIds.length > 0
      ? review!.map(entry => ({ memoryId: entry.memoryId, revisionId: entry.revisionId }))
      : [];
    const projectContext = projectChoices.length > 0
      ? {
          projectId: selectedProjectId,
          items: projectReview!.map(entry =>
            entry.kind === 'memory'
              ? {
                  kind: 'memory',
                  memoryId: entry.memoryId as string,
                  revisionId: entry.revisionId as string,
                }
              : {
                  kind: 'artifact',
                  artifactId: entry.artifactId as string,
                  version: entry.version,
                },
          ),
        }
      : undefined;
    startingRef.current = true;
    setStarting(true);
    setResult(null);
    setSavedArtifact('');
    setValidating(false);
    let started;
    try {
      started = await controller().start({
        projectId,
        beamA,
        beamB,
        controls,
        memoryContext,
        projectContext,
        // Only the currently attributed sources are sent;
        // an edited beam carries no document record, and
        // the backend re-verifies every record against
        // the submitted text before any provider traffic.
        documentA: documentA ?? undefined,
        documentB: documentB ?? undefined,
        urlA: urlA ?? undefined,
        urlB: urlB ?? undefined,
        zeroCostLock: lock,
      });
    } catch (e) {
      // A refused or unreachable start must be visible,
      // not a silent no-op: the run alert below renders it.
      if (mountedRef.current) setRun({
        runId: '',
        status: 'failed',
        partialChars: 0,
        error: e instanceof Error ? e.message : 'Starting the collision failed.',
      });
      return;
    } finally {
      startingRef.current = false;
      if (mountedRef.current) setStarting(false);
    }
    if (!mountedRef.current || !started) return; // superseded while the start was in flight
    setRun({ runId: started.runId, status: 'running', partialChars: 0, error: null });
    if (started.disclosure) setDisclosure(started.disclosure);
  }

  async function cancelRun() {
    if (!run) return;
    const polled = await controller().cancel();
    if (!polled) return;
    setRun({
      runId: polled.runId,
      status: polled.status,
      partialChars: polled.partialChars,
      error: polled.error ?? null,
    });
  }

  async function saveRun() {
    if (!run) return;
    try {
      const saved = await client.saveProjectCollider(workspaceId, run.runId);
      setSavedArtifact(saved.artifact.artifactId);
      onSaved();
      notify(
        saved.alreadySaved
          ? 'This collision was already saved as an artifact; no duplicate was created.'
          : `Collision saved as artifact ${saved.artifact.artifactId}.`,
        false,
      );
    } catch (e) {
      notify(e instanceof Error ? e.message : 'Saving the collision failed.', true);
    }
  }

  const running = run?.status === 'running' || run?.status === 'cancelling';
  return (
    <div>
      <p className="help">
        Collide two ideas, technologies, principles or industries and explore
        useful combinations. Supplied text is treated as private data, never as
        instructions to the model. Nothing starts until you press Collide Ideas.
        Saved-project selection is available below, as is .txt/.md
        document input for either beam, and URL fetching for either beam.
        3D visualization remains deferred.
      </p>
      <ColliderPipelineOverview
        state={pipelineStateForRun(run, validating)}
        partialChars={run?.partialChars ?? 0}
        error={run?.error ?? null}
        hasDisclosure={disclosure !== null}
        disclosureId="collider-providers-disclosure-title"
      />
      <form
        onSubmit={e => {
          e.preventDefault();
          void collide();
        }}
      >
        <label htmlFor="collider-beam-a">Beam A</label>
        <textarea
          id="collider-beam-a"
          value={beamA}
          onChange={e => editBeam('a', e.target.value)}
          required
          maxLength={8000}
          rows={5}
        />
        <label htmlFor="collider-document-a">
          Document for the first beam (.txt or .md)
        </label>
        <input
          id="collider-document-a"
          type="file"
          accept=".txt,.md,text/plain,text/markdown"
          onChange={e => {
            const file = e.target.files?.[0];
            // Reset so reselecting the same file name
            // with changed content fires again.
            e.target.value = '';
            if (file) void pickDocument('a', file);
          }}
        />
        {documentALoading && <p>Loading document…</p>}
        {documentAError && <p role="alert">{documentAError}</p>}
        {documentA && (
          <p>
            Document: {documentA.label} ·{' '}
            {documentA.kind === 'md' ? 'Markdown' : 'Plain text'} ·{' '}
            {documentA.rawBytes} bytes · {documentA.textChars} characters ·{' '}
            Text SHA-256:{' '}
            <span
              title={`Text SHA-256 ${documentA.textHash}\nFile SHA-256 ${documentA.rawHash}`}
            >
              {documentA.textHash.slice(0, 16)}…
            </span>
            {documentA.bom ? ' · leading BOM stripped' : ''}. Editing this
            beam clears this attribution.{' '}
            <button type="button" onClick={() => clearDocument('a')}>
              Clear document source
            </button>
          </p>
        )}
        <label htmlFor="collider-url-a">URL for the first beam</label>
        <input
          id="collider-url-a"
          type="text"
          inputMode="url"
          autoComplete="off"
          placeholder="https://example.com/page"
          value={urlAInput}
          onChange={e => setUrlAInput(e.target.value)}
        />
        <button
          type="button"
          onClick={() => void fetchUrl('a')}
        >
          Fetch for the first beam
        </button>
        {urlALoading && (
          <button type="button" onClick={() => cancelUrlFetch('a')}>
            Cancel fetch for the first beam
          </button>
        )}
        {urlALoading && <p>Fetching URL…</p>}
        {urlAError && <p role="alert">{urlAError}</p>}
        {urlA && (
          <p>
            URL: {urlA.submittedUrl}
            {urlA.finalUrl !== urlA.submittedUrl
              ? ` · final URL: ${urlA.finalUrl}`
              : ''}{' '}
            · {urlA.contentKind === 'html' ? 'Web page' : 'Plain text'}
            {urlA.contentKind === 'html'
              ? ' · conservatively extracted text'
              : ''}{' '}
            · {urlA.fetchedBytes} bytes fetched · {urlA.textChars} characters ·{' '}
            Text SHA-256:{' '}
            <span
              title={`Submitted URL ${urlA.submittedUrl}\nFinal URL ${urlA.finalUrl}\nText SHA-256 ${urlA.textHash}\nFetched SHA-256 ${urlA.fetchedHash}`}
            >
              {urlA.textHash.slice(0, 16)}…
            </span>
            {urlA.bom ? ' · leading BOM stripped' : ''}. Fetched content
            is untrusted data; editing this beam clears this attribution.{' '}
            <button type="button" onClick={() => clearUrlSource('a')}>
              Clear URL source for the first beam
            </button>
          </p>
        )}
        <label htmlFor="collider-beam-b">Beam B</label>
        <textarea
          id="collider-beam-b"
          value={beamB}
          onChange={e => editBeam('b', e.target.value)}
          required
          maxLength={8000}
          rows={5}
        />
        <label htmlFor="collider-document-b">
          Document for the second beam (.txt or .md)
        </label>
        <input
          id="collider-document-b"
          type="file"
          accept=".txt,.md,text/plain,text/markdown"
          onChange={e => {
            const file = e.target.files?.[0];
            e.target.value = '';
            if (file) void pickDocument('b', file);
          }}
        />
        {documentBLoading && <p>Loading document…</p>}
        {documentBError && <p role="alert">{documentBError}</p>}
        {documentB && (
          <p>
            Document: {documentB.label} ·{' '}
            {documentB.kind === 'md' ? 'Markdown' : 'Plain text'} ·{' '}
            {documentB.rawBytes} bytes · {documentB.textChars} characters ·{' '}
            Text SHA-256:{' '}
            <span
              title={`Text SHA-256 ${documentB.textHash}\nFile SHA-256 ${documentB.rawHash}`}
            >
              {documentB.textHash.slice(0, 16)}…
            </span>
            {documentB.bom ? ' · leading BOM stripped' : ''}. Editing this
            beam clears this attribution.{' '}
            <button type="button" onClick={() => clearDocument('b')}>
              Clear document source
            </button>
          </p>
        )}
        <label htmlFor="collider-url-b">URL for the second beam</label>
        <input
          id="collider-url-b"
          type="text"
          inputMode="url"
          autoComplete="off"
          placeholder="https://example.com/page"
          value={urlBInput}
          onChange={e => setUrlBInput(e.target.value)}
        />
        <button
          type="button"
          onClick={() => void fetchUrl('b')}
        >
          Fetch for the second beam
        </button>
        {urlBLoading && (
          <button type="button" onClick={() => cancelUrlFetch('b')}>
            Cancel fetch for the second beam
          </button>
        )}
        {urlBLoading && <p>Fetching URL…</p>}
        {urlBError && <p role="alert">{urlBError}</p>}
        {urlB && (
          <p>
            URL: {urlB.submittedUrl}
            {urlB.finalUrl !== urlB.submittedUrl
              ? ` · final URL: ${urlB.finalUrl}`
              : ''}{' '}
            · {urlB.contentKind === 'html' ? 'Web page' : 'Plain text'}
            {urlB.contentKind === 'html'
              ? ' · conservatively extracted text'
              : ''}{' '}
            · {urlB.fetchedBytes} bytes fetched · {urlB.textChars} characters ·{' '}
            Text SHA-256:{' '}
            <span
              title={`Submitted URL ${urlB.submittedUrl}\nFinal URL ${urlB.finalUrl}\nText SHA-256 ${urlB.textHash}\nFetched SHA-256 ${urlB.fetchedHash}`}
            >
              {urlB.textHash.slice(0, 16)}…
            </span>
            {urlB.bom ? ' · leading BOM stripped' : ''}. Fetched content
            is untrusted data; editing this beam clears this attribution.{' '}
            <button type="button" onClick={() => clearUrlSource('b')}>
              Clear URL source for the second beam
            </button>
          </p>
        )}
        <fieldset>
          <legend>Memory context (optional)</legend>
          <p className="help">
            Nothing is selected automatically. Selecting, reviewing or
            changing context starts no inference; only the memories you
            check, at the revisions shown below, are sent when you press
            Collide Ideas.
          </p>
          {contextMemories.length === 0 && (
            <p className="help">No other memories in this workspace yet.</p>
          )}
          {selectedIds.length >= 8 && (
            <p className="help">8 memories maximum for memory context.</p>
          )}
          {contextMemories.map(m => {
            const checked = selectedIds.includes(m.id);
            const inProjectContext = projectChoices.some(
              c => c.kind === 'memory' && c.id === m.id,
            );
            const disabled =
              !checked && (inProjectContext || selectedIds.length >= 8);
            return (
              <label key={m.id}>
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={disabled}
                  onChange={e => {
                    // Invalidate in the input event, before an old read can
                    // settle or an immediate submit can reuse the previous review.
                    reviewEpoch.current += 1;
                    setReview(null);
                    setReviewError('');
                    setSelectedIds(previous =>
                      e.target.checked
                        ? [...previous, m.id]
                        : previous.filter(id => id !== m.id),
                    );
                  }}
                />{' '}
                {m.title} · Revision {m.version} · {m.privacy}
                {inProjectContext && ' · already selected in project context'}
                {!checked && !inProjectContext && selectedIds.length >= 8 && ' · 8 maximum'}
              </label>
            );
          })}
        </fieldset>
        {selectedIds.length > 0 && (
          <section aria-label="Memory context to send">
            <p className="help">
              Exactly these revisions will be sent. Nothing is sent until
              you press Collide Ideas.
            </p>
            {reviewLoading && <p role="status">Loading the selected memory review…</p>}
            {reviewError && <p role="alert">{reviewError}</p>}
            {reviewError && (
              <button type="button" onClick={() => setMemoryReviewRetry(r => r + 1)}>
                Retry loading the memory review
              </button>
            )}
            {review && !reviewBoundsOk && (
              <p role="alert">
                A reviewed memory exceeds the 8000-character item limit, or
                the memory context exceeds 32000 characters in total;
                remove an item before colliding. Nothing is truncated.
              </p>
            )}
            {review &&
              review.map(entry => (
                <details key={entry.memoryId}>
                  <summary>
                    {entry.title} · revision {entry.version} · {entry.privacy}
                  </summary>
                  <pre>{entry.body}</pre>
                </details>
              ))}
          </section>
        )}
        <fieldset>
          <legend>Project context (optional)</legend>
          <p className="help">
            Choose a saved project and exactly which of its items to send.
            Nothing is selected automatically, and the project&apos;s own
            record text is never included. Selecting, reviewing or changing
            context starts no inference; only the items you check, at the
            versions shown below, are sent when you press Collide Ideas.
          </p>
          <label>
            Saved project
            <select
              id="collider-project"
              value={selectedProjectId}
              onChange={e => changeSelectedProject(e.target.value)}
            >
              <option value="">No project context</option>
              {projectOptions.map(p => (
                <option key={p.id} value={p.id}>
                  {p.title}
                  {p.id === projectId ? ' (this project)' : ''}
                </option>
              ))}
            </select>
          </label>
          {projectOptionsError && <p role="alert">{projectOptionsError}</p>}
          {projectOptionsError && (
            <button type="button" onClick={() => setProjectListRetry(r => r + 1)}>
              Retry loading saved projects
            </button>
          )}
          {projectMembersError && <p role="alert">{projectMembersError}</p>}
          {projectArtifactsError && <p role="alert">{projectArtifactsError}</p>}
          {(projectMembersError || projectArtifactsError) && (
            <button type="button" onClick={() => setProjectListRetry(r => r + 1)}>
              Retry loading project items
            </button>
          )}
          {projectHiddenMembers > 0 && (
            <p className="help">
              {projectHiddenMembers} project record(s) in this project are
              containers, not content, and are not offered.
            </p>
          )}
          {projectHiddenArtifacts > 0 && (
            <p className="help">
              {projectHiddenArtifacts} artifact(s) have ineligible types
              (approval records are never offered) and are not shown.
            </p>
          )}
          {projectChoices.length >= 8 && (
            <p className="help">8 items maximum for project context.</p>
          )}
          {selectedProjectId && projectListsLoaded && !projectMembersError && !projectArtifactsError && projectMembers.length === 0 && projectArtifacts.length === 0 && (
            <p className="help">This project has no selectable items yet.</p>
          )}
          {projectMembers.map(m => {
            const choice = projectChoices.find(
              c => c.kind === 'memory' && c.id === m.id,
            );
            const versions = projectVersions[`memory:${m.id}`];
            const inMemoryContext = selectedIds.includes(m.id);
            return (
              <div key={`memory:${m.id}`}>
                <label>
                  <input
                    type="checkbox"
                    checked={choice !== undefined}
                    disabled={
                      choice === undefined &&
                      (inMemoryContext || projectChoices.length >= 8)
                    }
                    onChange={e => toggleProjectMemory(m, e.target.checked)}
                  />{' '}
                  {m.title} · memory · Revision {m.version} · {m.privacy}
                  {inMemoryContext && ' · already selected in memory context'}
                  {choice === undefined && !inMemoryContext && projectChoices.length >= 8 && ' · 8 maximum'}
                </label>
                {choice && versions && versions.length > 0 && (
                  <label>
                    Version
                    <select
                      aria-label={`Version for ${m.title}`}
                      value={choice.versionId}
                      onChange={e => {
                        const option = versions.find(v => v.id === e.target.value);
                        if (option) {
                          changeProjectChoiceVersion(choice, option.id, option.version);
                        }
                      }}
                    >
                      {versions.map(v => (
                        <option key={v.id} value={v.id}>
                          Revision {v.version}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </div>
            );
          })}
          {projectArtifacts.map(a => {
            const choice = projectChoices.find(
              c => c.kind === 'artifact' && c.id === a.artifactId,
            );
            const versions = projectVersions[`artifact:${a.artifactId}`];
            return (
              <div key={`artifact:${a.artifactId}`}>
                <label>
                  <input
                    type="checkbox"
                    checked={choice !== undefined}
                    disabled={choice === undefined && projectChoices.length >= 8}
                    onChange={e => toggleProjectArtifact(a, e.target.checked)}
                  />{' '}
                  {a.title} · artifact ({a.type}) · Version {a.version}
                  {choice === undefined && projectChoices.length >= 8 && ' · 8 maximum'}
                </label>
                {choice && versions && versions.length > 0 && (
                  <label>
                    Version
                    <select
                      aria-label={`Version for ${a.title}`}
                      value={choice.versionId}
                      onChange={e => {
                        const option = versions.find(v => v.id === e.target.value);
                        if (option) {
                          changeProjectChoiceVersion(choice, option.id, option.version);
                        }
                      }}
                    >
                      {versions.map(v => (
                        <option key={v.id} value={v.id}>
                          Version {v.version}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </div>
            );
          })}
        </fieldset>
        {projectChoices.length > 0 && (
          <section aria-label="Project context to send">
            <p className="help">
              Exactly these project items, at these versions, will be sent.
              Nothing is sent until you press Collide Ideas.
            </p>
            {projectReviewLoading && (
              <p role="status">Loading the selected project review…</p>
            )}
            {projectReviewError && <p role="alert">{projectReviewError}</p>}
            {projectReviewError && (
              <button type="button" onClick={() => setProjectReviewRetry(r => r + 1)}>
                Retry loading the project review
              </button>
            )}
            {projectReview && !projectReviewBoundsOk && (
              <p role="alert">
                A reviewed project item exceeds the 8000-character item
                limit, or the project context exceeds 32000 characters in
                total; remove an item before colliding. Nothing is truncated.
              </p>
            )}
            {projectReview &&
              projectReview.map(entry => (
                <details
                  key={
                    entry.kind === 'memory'
                      ? `memory:${entry.memoryId}`
                      : `artifact:${entry.artifactId}`
                  }
                >
                  <summary>
                    {entry.title} ·{' '}
                    {entry.kind === 'memory' ? 'memory revision' : 'artifact version'}{' '}
                    {entry.version} · {entry.privacy}
                  </summary>
                  <pre>{entry.body}</pre>
                </details>
              ))}
          </section>
        )}
        <fieldset>
          <legend>Advanced controls</legend>
          <p className="help">
            Defaults work without configuration. These controls steer the
            model&apos;s emphasis only; they do not guarantee novelty,
            feasibility or correctness.
          </p>
          <label>
            Creativity — how far the combinations may stray from the obvious.
            <select
              id="collider-creativity"
              value={controls.creativity}
              onChange={e => setControls(c => ({ ...c, creativity: e.target.value }))}
            >
              <option value="low">Low</option>
              <option value="balanced">Balanced</option>
              <option value="high">High</option>
            </select>
          </label>
          <label>
            Technical depth — how detailed the technical discussion should be.
            <select
              id="collider-technical-depth"
              value={controls.technicalDepth}
              onChange={e => setControls(c => ({ ...c, technicalDepth: e.target.value }))}
            >
              <option value="overview">Overview</option>
              <option value="standard">Standard</option>
              <option value="deep">Deep</option>
            </select>
          </label>
          <label>
            Practicality — whether to favor speculative or workable proposals.
            <select
              id="collider-practicality"
              value={controls.practicality}
              onChange={e => setControls(c => ({ ...c, practicality: e.target.value }))}
            >
              <option value="speculative">Speculative</option>
              <option value="balanced">Balanced</option>
              <option value="practical">Practical</option>
            </select>
          </label>
          <label>
            Innovation distance — how daring the cross-domain leaps may be.
            <select
              id="collider-innovation-distance"
              value={controls.innovationDistance}
              onChange={e => setControls(c => ({ ...c, innovationDistance: e.target.value }))}
            >
              <option value="near">Near</option>
              <option value="moderate">Moderate</option>
              <option value="far">Far</option>
            </select>
          </label>
          <label>
            Preferred output type — which productization emphasis to favor.
            <select
              id="collider-output-type"
              value={controls.preferredOutputType}
              onChange={e =>
                setControls(c => ({ ...c, preferredOutputType: e.target.value }))
              }
            >
              <option value="concept">Concept</option>
              <option value="architecture">Architecture</option>
              <option value="product">Product</option>
              <option value="research">Research</option>
            </select>
          </label>
          <label htmlFor="collider-budget">
            Budget — a constraint you state (for example &quot;hobby
            budget&quot;); it is not a cost promise.
          </label>
          <input
            id="collider-budget"
            value={controls.budget}
            onChange={e => setControls(c => ({ ...c, budget: e.target.value }))}
            maxLength={200}
            autoComplete="off"
          />
          <label>
            <input
              type="checkbox"
              checked={lock}
              onChange={e => setLock(e.target.checked)}
            />{' '}
            Zero-Cost Lock (never spend)
          </label>
        </fieldset>
        <button type="submit" disabled={busy || running || starting || documentALoading || documentBLoading || urlALoading || urlBLoading || !reviewReady || !projectReviewReady || !reviewBoundsOk || !projectReviewBoundsOk}>
          Collide Ideas
        </button>
        {run?.status === 'running' && (
          <button type="button" className="secondary" onClick={() => void cancelRun()}>
            Cancel collision
          </button>
        )}
      </form>
      {starting && <p role="status">Starting collision…</p>}
      {run?.status === 'running' && (
        <p role="status">Colliding… {run.partialChars} characters received.</p>
      )}
      {run?.status === 'cancelling' && (
        <p role="status">Cancelling… waiting for the provider stream to stop.</p>
      )}
      {run?.status === 'cancelled' && (
        <p role="status">Collision cancelled. Nothing was saved.</p>
      )}
      {run && (run.status === 'failed' || run.status === 'invalid') && (
        <p role="alert">
          Collision {run.status}: {run.error ?? 'the provider did not produce a usable result.'}{' '}
          Nothing was saved; you can collide again.
        </p>
      )}
      {validating && <p role="status">Validating output…</p>}
      {disclosure && (
        <DisclosurePanel
          disclosure={disclosure}
          headingId="collider-providers-disclosure-title"
        />
      )}
      {result && (
        <div>
          <ColliderResultView result={result} />
          {savedArtifact ? (
            <p role="status">Saved as artifact {savedArtifact} in this project.</p>
          ) : (
            <button type="button" disabled={busy} onClick={() => void saveRun()}>
              Save collision as experiment
            </button>
          )}
        </div>
      )}
    </div>
  );
}
