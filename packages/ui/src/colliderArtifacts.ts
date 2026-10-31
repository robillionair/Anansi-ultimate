// M8 §27 versioned experiment artifacts — TypeScript
// mirror of `crates/core/src/collider_artifacts.rs`.
// Byte-identical by construction: same line arrays,
// same order, same trailing-newline rule. The Rust
// suite pins hashes for the shared fixture; the node
// suite asserts the same hashes here. If either side
// drifts, the two suites disagree.
//
// The input type is structural: any object with the
// five fields below derives the same documents, so
// the strict parser can verify byte counts before
// the full document type exists.
import type { ColliderResult } from '@anansi/contracts';

export interface ArtifactSourceDoc {
  beamA: string;
  beamB: string;
  provider: string;
  model: string;
  result: ColliderResult;
}

export const ARTIFACT_FILES = [
  'CONCEPT.md',
  'ARCHITECTURE.md',
  'FEASIBILITY.md',
  'ROADMAP.md',
  'RISKS.md',
  'TEST_PLAN.md',
] as const;

export type ArtifactFile = (typeof ARTIFACT_FILES)[number];

const NOT_STATED = 'Not stated in the source record.';
const HEADER_NOTE =
  'Derived from a saved Collider experiment record. This document organizes the record\'s own fields; it adds no new claims. Anything the record does not state is marked "Not stated in the source record."';

function singleLine(value: string): string {
  return value.split(/\s+/).filter(Boolean).join(' ');
}

function header(doc: ArtifactSourceDoc, title: string): string[] {
  return [
    `# ${title}`,
    '',
    `${HEADER_NOTE} (provider ${singleLine(doc.provider)}, model ${singleLine(doc.model)}).`,
    '',
  ];
}

function bullets(items: string[]): string[] {
  if (!items.length) return ['None recorded.'];
  return items.map(item => `- ${item}`);
}

function section(title: string, body: string[]): string[] {
  return [`## ${title}`, '', ...body, ''];
}

function subsection(title: string, body: string[]): string[] {
  return [`### ${title}`, '', ...body, ''];
}

function finish(lines: string[]): string {
  return `${lines.join('\n').replace(/\n+$/, '')}\n`;
}

function concept(doc: ArtifactSourceDoc): string {
  const lines = header(doc, 'Concept');
  lines.push(...section('Beams', []));
  lines.push(...subsection('Beam A', [doc.beamA]));
  lines.push(...subsection('Beam B', [doc.beamB]));
  lines.push(...section('Source understanding', [doc.result.sourceUnderstanding]));
  lines.push(...section('Domain intersections', bullets(doc.result.crossDomainMappings)));
  lines.push(...section('Conceptual primitives', bullets(doc.result.conceptualPrimitives)));
  const combinations: string[] = [];
  for (const combination of doc.result.combinations) {
    combinations.push(...subsection(combination.name, [combination.description]));
  }
  lines.push(...section('Proposed combinations', combinations));
  lines.push(...section('Concept brief', [doc.result.productization.conceptBrief]));
  lines.push(
    ...section('Unique value proposition', [
      'The record states no separate value-proposition statement. The concept brief above is the record\'s own statement of value.',
    ]),
  );
  return finish(lines);
}

function architecture(doc: ArtifactSourceDoc): string {
  const lines = header(doc, 'Architecture');
  lines.push(...section('Technical architecture', [doc.result.productization.architecture]));
  const blocks = bullets(doc.result.conceptualPrimitives);
  blocks.push('Note: these are conceptual primitives from the record, not a module list.');
  lines.push(...section('Conceptual building blocks', blocks));
  lines.push(...section('Module decomposition', [NOT_STATED]));
  lines.push(...section('Technology stack options', [NOT_STATED]));
  lines.push(...section('Interfaces', [NOT_STATED]));
  return finish(lines);
}

function feasibility(doc: ArtifactSourceDoc): string {
  const f = doc.result.feasibility;
  const lines = header(doc, 'Feasibility');
  lines.push(...section('Hypotheses', bullets(f.hypotheses)));
  lines.push(...section('Dependencies', bullets(f.dependencies)));
  lines.push(...section('Unknown costs', bullets(f.unknownCosts)));
  lines.push(...section('Risks', bullets(f.risks)));
  lines.push(...section('Testability', [f.testability]));
  lines.push(
    ...section('Complexity hotspots', [
      'The record names no separate complexity hotspots. The dependencies above are the record\'s own complexity inputs.',
    ]),
  );
  lines.push(...section('Performance bottlenecks', [NOT_STATED]));
  lines.push(...section('Mitigation strategies', [NOT_STATED]));
  return finish(lines);
}

function roadmap(doc: ArtifactSourceDoc): string {
  const p = doc.result.productization;
  const lines = header(doc, 'Roadmap');
  lines.push(...section('Delivery timeline', [p.roadmap]));
  lines.push(
    ...section('Phases', [
      'The record states no separate phase breakdown; the delivery timeline above is the record\'s own timeline.',
    ]),
  );
  lines.push(...section('Critical milestones', [NOT_STATED]));
  lines.push(...section('Dependencies', bullets(doc.result.feasibility.dependencies)));
  const inputs: string[] = [];
  inputs.push(...subsection('Test plan', [p.testPlan]));
  inputs.push(...subsection('Prototype proposal', [p.prototypeProposal]));
  lines.push(...section('Delivery inputs', inputs));
  return finish(lines);
}

function risks(doc: ArtifactSourceDoc): string {
  const lines = header(doc, 'Risk register');
  const rows = doc.result.feasibility.risks.map(
    risk =>
      `- ${risk} — Severity: ${NOT_STATED} Likelihood: ${NOT_STATED} Mitigation: ${NOT_STATED}`,
  );
  lines.push(...section('Risks', bullets(rows)));
  const costs = bullets(doc.result.feasibility.unknownCosts);
  costs.push('These are uncertain costs from the record, not risks.');
  lines.push(...section('Uncertain costs', costs));
  lines.push(
    ...section('Risk categories', [
      'The record assigns no categories to its risks. The register above lists every recorded risk without invented categories.',
    ]),
  );
  return finish(lines);
}

function testPlan(doc: ArtifactSourceDoc): string {
  const p = doc.result.productization;
  const lines = header(doc, 'Test plan');
  const acceptance: string[] = [];
  for (const combination of doc.result.combinations) {
    acceptance.push(
      ...subsection(`Acceptance: ${combination.name}`, [
        combination.description,
        '',
        `Pass criterion: ${NOT_STATED}`,
      ]),
    );
  }
  lines.push(...section('Acceptance test specifications', acceptance));
  lines.push(...section('Test plan', [p.testPlan]));
  lines.push(...section('Testability', [doc.result.feasibility.testability]));
  const edge = [
    'Edge-case inputs from the record\'s own risks (each risk names a failure mode to test):',
    ...bullets(doc.result.feasibility.risks),
  ];
  lines.push(...section('Edge cases', edge));
  lines.push(...section('Adversarial inputs', [NOT_STATED]));
  lines.push(...section('Stress scenarios', [NOT_STATED]));
  lines.push(...section('Performance budgets', [NOT_STATED]));
  return finish(lines);
}

export function deriveArtifact(doc: ArtifactSourceDoc, file: string): string | null {
  switch (file) {
    case 'CONCEPT.md':
      return concept(doc);
    case 'ARCHITECTURE.md':
      return architecture(doc);
    case 'FEASIBILITY.md':
      return feasibility(doc);
    case 'ROADMAP.md':
      return roadmap(doc);
    case 'RISKS.md':
      return risks(doc);
    case 'TEST_PLAN.md':
      return testPlan(doc);
    default:
      return null;
  }
}

export function deriveAllArtifacts(doc: ArtifactSourceDoc): Array<{ file: ArtifactFile; text: string }> {
  const out: Array<{ file: ArtifactFile; text: string }> = [];
  for (const file of ARTIFACT_FILES) {
    const text = deriveArtifact(doc, file);
    if (text !== null) out.push({ file, text });
  }
  return out;
}

export function utf8Bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}
