/**
 * M8 visualization: derive a scene graph from an exact
 * saved Collider experiment document. This module is
 * pure — no I/O, no API calls, no randomness — so the
 * same pinned version always derives the same scene,
 * which is what makes restart/restore comparisons
 * meaningful.
 *
 * Honesty rule for edges: ONLY section flow plus
 * section membership. Section-flow edges reuse the
 * pipeline relations from ColliderOverview verbatim
 * (beams ground the understanding; primitives are
 * distilled from it; mappings connect primitives in
 * the pipeline order; combinations are built from the
 * mappings; feasibility reviews the combinations;
 * productization carries the path forward). No edge is
 * mined from free text: a mapping node does NOT link
 * to particular primitives, and a feasibility node does
 * NOT link to particular combinations, because the
 * saved record does not say which. Item nodes belong
 * to their section; that membership is in the data.
 *
 * Rendering is deliberately renderer-agnostic: the
 * component draws this scene on Canvas 2D today, and a
 * Three.js/WebGL renderer can replace that drawing
 * later without changing derivation, layout, selection,
 * or provenance (see the M8 visualization design).
 * All text reaches the screen via React text nodes or
 * canvas fillText — never via innerHTML — so saved
 * experiment text is displayed, never executed.
 */

import type { ColliderExperimentDoc } from '@anansi/contracts';

export type VisualizationSection =
  | 'beam-a'
  | 'beam-b'
  | 'understanding'
  | 'primitives'
  | 'mappings'
  | 'combinations'
  | 'feasibility'
  | 'productization';

export type VisualizationNode = {
  id: string;
  section: VisualizationSection;
  /** Section hub (the ring position) or an item orbiting it. */
  role: 'section' | 'item';
  /** Short display label, cut from the node's own text (or its exact name). */
  label: string;
  /** Exact source text shown when the node is selected. */
  text: string;
  /** Provenance lines shown with the text on selection. */
  provenance: string[];
};

export type VisualizationEdge = {
  from: string;
  to: string;
  /** The pipeline relation this edge states, verbatim from the overview. */
  relation: string;
};

export type VisualizationScene = {
  /** Pinned identity: `${artifactId}@v${version}`. */
  identity: string;
  artifactId: string;
  version: number;
  provider: string;
  model: string;
  nodes: VisualizationNode[];
  edges: VisualizationEdge[];
};

export const VISUALIZATION_SECTIONS: {
  section: VisualizationSection;
  title: string;
  relation: string;
}[] = [
  {
    section: 'beam-a',
    title: 'Beam A',
    relation: 'First supplied beam; grounds the understanding with Beam B.',
  },
  {
    section: 'beam-b',
    title: 'Beam B',
    relation: 'Second supplied beam; grounds the understanding with Beam A.',
  },
  {
    section: 'understanding',
    title: 'Source understanding',
    relation: 'The shared patterns across the two beams; grounds everything below.',
  },
  {
    section: 'primitives',
    title: 'Conceptual primitives',
    relation: 'Distilled concepts from that understanding.',
  },
  {
    section: 'mappings',
    title: 'Cross-domain mappings',
    relation: 'Connections between the primitives across the two domains.',
  },
  {
    section: 'combinations',
    title: 'Proposed combinations',
    relation: 'Built from the mappings (at least two).',
  },
  {
    section: 'feasibility',
    title: 'Feasibility',
    relation:
      'Hypotheses, dependencies, unknown costs, risks and testability for the combinations.',
  },
  {
    section: 'productization',
    title: 'Productization',
    relation:
      'Brief, architecture, roadmap, test plan and prototype proposal for the path forward.',
  },
];

const SECTION_ORDER: VisualizationSection[] = VISUALIZATION_SECTIONS.map(
  s => s.section,
);

function sectionTitle(section: VisualizationSection): string {
  return VISUALIZATION_SECTIONS.find(s => s.section === section)?.title ?? section;
}

function sectionRelation(section: VisualizationSection): string {
  return (
    VISUALIZATION_SECTIONS.find(s => s.section === section)?.relation ?? ''
  );
}

/** Display label: first line, whitespace-joined, cut to 60 chars. */
function shortLabel(text: string): string {
  const joined = text.split(/\s+/).filter(Boolean).join(' ');
  return joined.length > 60 ? `${joined.slice(0, 57)}…` : joined;
}

function beamProvenance(
  doc: ColliderExperimentDoc,
  beam: 'a' | 'b',
): string[] {
  const source = beam === 'a' ? doc.documentA : doc.documentB;
  const url = beam === 'a' ? doc.urlA : doc.urlB;
  const lines: string[] = [];
  if (source) {
    lines.push(
      `Document: ${source.label} · ${source.kind === 'md' ? 'Markdown' : 'Plain text'} · ${source.rawBytes} bytes · ${source.textChars} characters · text SHA-256 ${source.textHash}${source.bom ? ' · leading BOM stripped' : ''}. Generation-time record.`,
    );
  }
  if (url) {
    lines.push(
      `URL: ${url.submittedUrl}${url.finalUrl !== url.submittedUrl ? ` · final URL: ${url.finalUrl}` : ''} · ${url.contentKind === 'html' ? 'Web page, conservatively extracted text' : 'Plain text'} · ${url.fetchedBytes} bytes fetched · ${url.textChars} characters · text SHA-256 ${url.textHash}${url.bom ? ' · leading BOM stripped' : ''}. Generation-time record.`,
    );
  }
  return lines;
}

/**
 * Derive the scene for one pinned experiment version.
 * The doc is already strictly parsed by
 * parseColliderExperimentBody before viewing, so this
 * trusts its shape but never invents content: every
 * node text comes verbatim from the doc.
 */
export function deriveVisualizationScene(
  doc: ColliderExperimentDoc,
  artifactId: string,
  version: number,
): VisualizationScene {
  const baseProvenance = [
    `Experiment ${artifactId} version ${version} · provider ${doc.provider} · model ${doc.model}. Immutable saved record.`,
  ];
  const nodes: VisualizationNode[] = [];
  const push = (
    id: string,
    section: VisualizationSection,
    role: 'section' | 'item',
    label: string,
    text: string,
    provenance: string[],
  ) => {
    nodes.push({ id, section, role, label, text, provenance });
  };

  push(
    'beam-a',
    'beam-a',
    'section',
    'Beam A',
    doc.beamA,
    [...baseProvenance, ...beamProvenance(doc, 'a')],
  );
  push(
    'beam-b',
    'beam-b',
    'section',
    'Beam B',
    doc.beamB,
    [...baseProvenance, ...beamProvenance(doc, 'b')],
  );
  push(
    'understanding',
    'understanding',
    'section',
    'Source understanding',
    doc.result.sourceUnderstanding,
    baseProvenance,
  );

  push(
    'primitives',
    'primitives',
    'section',
    'Conceptual primitives',
    sectionRelation('primitives'),
    baseProvenance,
  );
  doc.result.conceptualPrimitives.forEach((primitive, i) => {
    push(
      `primitive-${i}`,
      'primitives',
      'item',
      shortLabel(primitive),
      primitive,
      baseProvenance,
    );
  });

  push(
    'mappings',
    'mappings',
    'section',
    'Cross-domain mappings',
    sectionRelation('mappings'),
    baseProvenance,
  );
  doc.result.crossDomainMappings.forEach((mapping, i) => {
    push(
      `mapping-${i}`,
      'mappings',
      'item',
      shortLabel(mapping),
      mapping,
      baseProvenance,
    );
  });

  push(
    'combinations',
    'combinations',
    'section',
    'Proposed combinations',
    sectionRelation('combinations'),
    baseProvenance,
  );
  doc.result.combinations.forEach((combination, i) => {
    push(
      `combination-${i}`,
      'combinations',
      'item',
      combination.name,
      combination.description,
      baseProvenance,
    );
  });

  push(
    'feasibility',
    'feasibility',
    'section',
    'Feasibility',
    sectionRelation('feasibility'),
    baseProvenance,
  );
  const feasibilityLists: [string, string][] = [
    ...doc.result.feasibility.hypotheses.map(
      (text, i): [string, string] => [`feasibility-hypothesis-${i}`, text],
    ),
    ...doc.result.feasibility.dependencies.map(
      (text, i): [string, string] => [`feasibility-dependency-${i}`, text],
    ),
    ...doc.result.feasibility.unknownCosts.map(
      (text, i): [string, string] => [`feasibility-unknown-cost-${i}`, text],
    ),
    ...doc.result.feasibility.risks.map(
      (text, i): [string, string] => [`feasibility-risk-${i}`, text],
    ),
  ];
  for (const [id, text] of feasibilityLists) {
    push(id, 'feasibility', 'item', shortLabel(text), text, baseProvenance);
  }
  push(
    'feasibility-testability',
    'feasibility',
    'item',
    'Testability',
    doc.result.feasibility.testability,
    baseProvenance,
  );

  push(
    'productization',
    'productization',
    'section',
    'Productization',
    sectionRelation('productization'),
    baseProvenance,
  );
  const productizationFields: [string, string, string][] = [
    ['productization-concept-brief', 'Concept brief', doc.result.productization.conceptBrief],
    ['productization-architecture', 'Architecture', doc.result.productization.architecture],
    ['productization-roadmap', 'Roadmap', doc.result.productization.roadmap],
    ['productization-test-plan', 'Test plan', doc.result.productization.testPlan],
    ['productization-prototype-proposal', 'Prototype proposal', doc.result.productization.prototypeProposal],
  ];
  for (const [id, label, text] of productizationFields) {
    push(id, 'productization', 'item', label, text, baseProvenance);
  }

  // Section flow only — see the module honesty rule.
  const flow: [string, string][] = [
    ['beam-a', 'understanding'],
    ['beam-b', 'understanding'],
    ['understanding', 'primitives'],
    ['primitives', 'mappings'],
    ['mappings', 'combinations'],
    ['combinations', 'feasibility'],
    ['feasibility', 'productization'],
  ];
  const edges: VisualizationEdge[] = flow.map(([from, to]) => ({
    from,
    to,
    relation:
      VISUALIZATION_SECTIONS.find(s => s.section === (to as VisualizationSection))
        ?.relation ?? '',
  }));

  return {
    identity: `${artifactId}@v${version}`,
    artifactId,
    version,
    provider: doc.provider,
    model: doc.model,
    nodes,
    edges,
  };
}

export type VisualizationPosition = { x: number; y: number; r: number };

export type VisualizationLayout = {
  width: number;
  height: number;
  positions: Map<string, VisualizationPosition>;
};

/**
 * Deterministic layout: section hubs on a ring around a
 * central collision point, item nodes orbiting their hub
 * at fixed angles. Pure function of the scene and the
 * viewport size — no randomness, no time — so the same
 * version always lays out identically.
 */
export function layoutVisualizationScene(
  scene: VisualizationScene,
  width: number,
  height: number,
): VisualizationLayout {
  const cx = width / 2;
  const cy = height / 2;
  const ring = Math.min(width, height) * 0.34;
  const positions = new Map<string, VisualizationPosition>();
  const hubAt = new Map<string, { x: number; y: number }>();
  SECTION_ORDER.forEach((section, i) => {
    const angle = -Math.PI / 2 + (i / SECTION_ORDER.length) * Math.PI * 2;
    const x = cx + ring * Math.cos(angle);
    const y = cy + ring * Math.sin(angle);
    hubAt.set(section, { x, y });
    const hub = scene.nodes.find(n => n.id === section);
    if (hub) positions.set(hub.id, { x, y, r: 26 });
  });
  for (const section of SECTION_ORDER) {
    const hub = hubAt.get(section);
    if (!hub) continue;
    const items = scene.nodes.filter(n => n.section === section && n.role === 'item');
    items.forEach((item, i) => {
      const angle = (i / Math.max(items.length, 1)) * Math.PI * 2;
      const orbit = 52;
      const x = Math.min(
        Math.max(hub.x + orbit * Math.cos(angle), 16),
        width - 16,
      );
      const y = Math.min(
        Math.max(hub.y + orbit * Math.sin(angle), 16),
        height - 16,
      );
      positions.set(item.id, { x, y, r: 12 });
    });
  }
  return { width, height, positions };
}

/**
 * Hit-test a point against the layout. Items are tested
 * before hubs because they draw on top; the first match
 * wins. Returns the node id or null.
 */
export function hitTestVisualization(
  layout: VisualizationLayout,
  x: number,
  y: number,
): string | null {
  // Items first (smaller, drawn above hubs).
  for (const [id, pos] of layout.positions) {
    if (pos.r <= 12) {
      const dx = x - pos.x;
      const dy = y - pos.y;
      if (dx * dx + dy * dy <= pos.r * pos.r) return id;
    }
  }
  for (const [id, pos] of layout.positions) {
    const dx = x - pos.x;
    const dy = y - pos.y;
    if (dx * dx + dy * dy <= pos.r * pos.r) return id;
  }
  return null;
}

export function sectionTitleFor(section: VisualizationSection): string {
  return sectionTitle(section);
}
