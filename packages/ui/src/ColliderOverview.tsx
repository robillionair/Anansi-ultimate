/**
 * M8 WP3: an accessible overview of the Collider's REAL
 * pipeline state. The backend makes ONE provider request
 * per collision, so there are no six live stages, no
 * per-section progress and no percentages: before a run
 * completes, every section shares the run's state, and
 * after validation all six are complete together. This
 * component renders only that honest vocabulary, a static
 * (never animated) diagram of how the six output sections
 * relate, the same relationships as a text list, and
 * anchor navigation to the rendered sections and to the
 * disclosure (provenance) section. It contains no
 * animation in any mode, so it complies with reduced
 * motion without a branch.
 */

export type ColliderPipelineState =
  | 'not-started'
  | 'generating'
  | 'validating'
  | 'complete'
  | 'failed'
  | 'invalid'
  | 'cancelling'
  | 'cancelled';

/**
 * Map the panel's run state to the overview's vocabulary.
 * Validating wins over the polled "complete" status: the
 * stream finished but the output has not passed
 * validation yet. Any other terminal status is failed;
 * "invalid" stays distinct because completed output that
 * fails validation is a different outcome from a failed
 * run, with a different reason to show.
 */
export function pipelineStateForRun(
  run: { status: string } | null,
  validating: boolean,
): ColliderPipelineState {
  if (!run) return 'not-started';
  if (validating) return 'validating';
  switch (run.status) {
    case 'running':
      return 'generating';
    case 'cancelling':
      return 'cancelling';
    case 'cancelled':
      return 'cancelled';
    case 'complete':
      return 'complete';
    case 'invalid':
      return 'invalid';
    default:
      return 'failed';
  }
}

export type ColliderPipelineSection = {
  /** Stable slug used to build an instance-scoped heading id. */
  slug: string;
  /** Legacy live-instance heading id (empty instance scope). */
  headingId: string;
  title: string;
  /** How this section relates to the others, in output order. */
  relation: string;
};

export const COLLIDER_PIPELINE_SECTIONS: ColliderPipelineSection[] = [
  {
    slug: 'understanding',
    headingId: 'collider-understanding-heading',
    title: 'Source understanding',
    relation: 'The shared patterns across the two beams; grounds everything below.',
  },
  {
    slug: 'primitives',
    headingId: 'collider-primitives-heading',
    title: 'Conceptual primitives',
    relation: 'Distilled concepts from that understanding.',
  },
  {
    slug: 'mappings',
    headingId: 'collider-mappings-heading',
    title: 'Cross-domain mappings',
    relation: 'Connections between the primitives across the two domains.',
  },
  {
    slug: 'combinations',
    headingId: 'collider-combinations-heading',
    title: 'Proposed combinations',
    relation: 'Built from the mappings (at least two).',
  },
  {
    slug: 'feasibility',
    headingId: 'collider-feasibility-heading',
    title: 'Feasibility',
    relation: 'Hypotheses, dependencies, unknown costs, risks and testability for the combinations.',
  },
  {
    slug: 'productization',
    headingId: 'collider-productization-heading',
    title: 'Productization',
    relation: 'Brief, architecture, roadmap, test plan and prototype proposal for the path forward.',
  },
];

/**
 * R3: the heading id for one result instance. An empty
 * scope is the live panel and keeps the legacy ids, so
 * live URLs and locators are unchanged. A saved view
 * passes `saved-{artifactId}-v{version}`, so its headings
 * (and the overview links that point at them) can never
 * resolve to the still-mounted live panel. The result
 * view must use this same function for its heading ids.
 */
export function colliderResultHeadingId(
  instanceScope: string | undefined,
  slug: string,
): string {
  return instanceScope
    ? `collider-${instanceScope}-${slug}-heading`
    : `collider-${slug}-heading`;
}

const STATE_LABELS: Record<ColliderPipelineState, string> = {
  'not-started': 'Not started',
  generating: 'Generating',
  validating: 'Validating',
  complete: 'Complete',
  failed: 'Failed',
  invalid: 'Invalid',
  cancelling: 'Cancelling',
  cancelled: 'Cancelled',
};

function stateText(
  state: ColliderPipelineState,
  partialChars: number,
  error: string | null,
): string {
  switch (state) {
    case 'not-started':
      return 'Not started — press Collide Ideas to make one provider request that produces all six sections below.';
    case 'generating':
      return `Generating — one provider request is in flight; ${partialChars} stream characters received so far. Sections arrive together when it completes; there is no per-section progress.`;
    case 'validating':
      return 'Validating — the completed output is being strictly re-parsed before anything renders.';
    case 'complete':
      return 'Complete — all six sections passed validation and render below.';
    case 'failed':
      return `Failed — ${error ?? 'the collision failed.'} Nothing was saved.`;
    case 'invalid':
      return `Invalid — the completed output did not pass validation and was not rendered. ${error ?? ''}`.trim();
    case 'cancelling':
      return 'Cancelling — waiting for the provider stream to stop.';
    case 'cancelled':
      return 'Cancelled — nothing was saved.';
  }
}

/** Short labels for the static diagram boxes. */
const DIAGRAM_LABELS = [
  'Source understanding',
  'Primitives',
  'Mappings',
  'Combinations',
  'Feasibility',
  'Productization',
];

export function ColliderPipelineOverview({
  state,
  partialChars = 0,
  error = null,
  hasDisclosure = false,
  instanceScope,
  disclosureId = 'providers-disclosure-title',
}: {
  state: ColliderPipelineState;
  partialChars?: number;
  error?: string | null;
  /** Render the provenance link only when a disclosure section exists on this page. */
  hasDisclosure?: boolean;
  /**
   * Instance scope for the section links (see
   * colliderResultHeadingId). Empty for the live panel;
   * a saved view passes its artifact scope.
   */
  instanceScope?: string;
  /** Heading id of this instance's disclosure panel. */
  disclosureId?: string;
}) {
  const stateLabel = STATE_LABELS[state];
  // R3 exception, deliberate: this heading id is not a
  // link target and stays at its legacy value in both
  // instances, because the saved-overview probe locates
  // the saved overview by this id inside the Artifacts
  // region. Navigation correctness is carried by the
  // six scoped section targets, not by this heading.
  return (
    <section aria-labelledby="collider-pipeline-heading">
      <h3 id="collider-pipeline-heading">Collision pipeline</h3>
      <p role="status">{stateText(state, partialChars, error)}</p>
      <p className="help">
        One provider request produces all six sections at once; they are the
        structure of a single model output, not six executed stages. A 3D
        presentation remains unfinished; the diagram below and the list
        after it describe the same relationships.
      </p>
      <nav aria-label="Collision sections">
        <ol>
          {COLLIDER_PIPELINE_SECTIONS.map(section => (
            <li key={section.headingId}>
              {state === 'complete' ? (
                <a href={`#${colliderResultHeadingId(instanceScope, section.slug)}`}>
                  {section.title}
                </a>
              ) : (
                <span>{section.title}</span>
              )}
              {` — ${stateLabel} — ${section.relation}`}
            </li>
          ))}
        </ol>
      </nav>
      <svg
        role="img"
        aria-label="Flow of the six collision sections in order: source understanding, primitives, mappings, combinations, feasibility, productization."
        viewBox="0 0 320 384"
        width="320"
        height="384"
      >
        {DIAGRAM_LABELS.map((label, i) => {
          const y = i * 64;
          return (
            <g key={label}>
              <rect x="20" y={y} width="280" height="44" fill="none" stroke="currentColor" />
              <text x="160" y={y + 27} textAnchor="middle" fill="currentColor">
                {label}
              </text>
              {i < DIAGRAM_LABELS.length - 1 && (
                <text x="160" y={y + 58} textAnchor="middle" fill="currentColor">
                  ↓
                </text>
              )}
            </g>
          );
        })}
      </svg>
      {hasDisclosure && (
        <p>
          <a href={`#${disclosureId}`}>
            View what ANANSI sent (provenance)
          </a>
        </p>
      )}
    </section>
  );
}
