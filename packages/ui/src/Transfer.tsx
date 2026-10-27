import { useRef, useState } from 'react';
import type {
  ArtifactRecord,
  Client,
  ColliderExperimentDoc,
  TransferConceptDoc,
} from '@anansi/contracts';

// ---------------------------------------------------------------------------
// Strict client-side parsing for transferred concepts (mirrors the
// backend bounds in crates/core/src/transfer.rs). An unrenderable
// body never renders as a partial concept.

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, keys: string[]): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return (
    actual.length === expected.length &&
    actual.every((key, i) => key === expected[i])
  );
}

function textField(value: unknown, max: number): string | null {
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
  max: number,
  maxChars: number,
): string[] | null {
  if (!Array.isArray(value)) return null;
  if (value.length < min || value.length > max) return null;
  const out: string[] = [];
  for (const item of value) {
    const text = textField(item, maxChars);
    if (text === null) return null;
    out.push(text);
  }
  return out;
}

export function parseTransferConceptBody(
  body: string,
): TransferConceptDoc | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;
  if (
    !exactKeys(parsed, [
      'format',
      'version',
      'sourceExperimentArtifactId',
      'sourceExperimentVersion',
      'sourceExperimentContentHash',
      'selectedCombinationIndex',
      'selectedCombination',
      'approvedBrief',
      'assumptions',
      'unknowns',
      'productizationBrief',
      'provider',
      'model',
    ])
  ) {
    return null;
  }
  if (parsed.format !== 'anansi-forge-concept' || parsed.version !== 1) {
    return null;
  }
  if (!isRecord(parsed.selectedCombination)) return null;
  if (!exactKeys(parsed.selectedCombination, ['name', 'description'])) return null;
  const name = textField(parsed.selectedCombination.name, 120);
  const description = textField(parsed.selectedCombination.description, 1200);
  const approvedBrief = textField(parsed.approvedBrief, 4000);
  const assumptions = textList(parsed.assumptions, 1, 8, 600);
  const unknowns = textList(parsed.unknowns, 0, 16, 300);
  const productizationBrief = textField(parsed.productizationBrief, 2000);
  if (
    name === null ||
    description === null ||
    approvedBrief === null ||
    assumptions === null ||
    unknowns === null ||
    productizationBrief === null ||
    typeof parsed.sourceExperimentArtifactId !== 'string' ||
    parsed.sourceExperimentArtifactId.trim() === '' ||
    typeof parsed.sourceExperimentVersion !== 'number' ||
    typeof parsed.sourceExperimentContentHash !== 'string' ||
    parsed.sourceExperimentContentHash.trim() === '' ||
    typeof parsed.selectedCombinationIndex !== 'number' ||
    typeof parsed.provider !== 'string' ||
    parsed.provider.trim() === '' ||
    typeof parsed.model !== 'string' ||
    parsed.model.trim() === ''
  ) {
    return null;
  }
  return parsed as unknown as TransferConceptDoc;
}

// ---------------------------------------------------------------------------
// Transferred concept view: pins, provenance, labeled origins.

export function TransferConceptView({ doc }: { doc: TransferConceptDoc }) {
  return (
    <section aria-label="Transferred concept">
      <p>
        Transferred concept — pinned to collider experiment{' '}
        {doc.sourceExperimentArtifactId} version {doc.sourceExperimentVersion}
        (content hash {doc.sourceExperimentContentHash.slice(0, 16)}…).
        Provider {doc.provider}, model {doc.model}. This record is immutable;
        a newer experiment version neither changes it nor inherits its
        approval.
      </p>
      <p>
        <strong>Selected combination (model-derived, verbatim).</strong>{' '}
        {doc.selectedCombination.name}: {doc.selectedCombination.description}
      </p>
      <p>
        <strong>Approved brief (user-authored).</strong>
      </p>
      <pre>{doc.approvedBrief}</pre>
      <p>
        <strong>Assumptions (model-derived, verbatim).</strong>
      </p>
      <ul>
        {doc.assumptions.map((item, i) => (
          <li key={i}>{item}</li>
        ))}
      </ul>
      <p>
        <strong>Unknowns (model-derived, verbatim).</strong>{' '}
        {doc.unknowns.length ? doc.unknowns.join('; ') : 'None stated.'}
      </p>
      <p>
        <strong>Productization brief (model-derived, verbatim).</strong>{' '}
        {doc.productizationBrief}
      </p>
      <p className="help">
        This concept is structurally validated, not factually verified.
        Transfer approval is not specification approval: generating a
        specification below is a separate explicit action, and nothing here
        authorizes paid inference or runs generated code.
      </p>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Transfer panel: select, preview, edit, explicitly approve. Rendering,
// selecting or editing calls no client method and saves nothing.

export function defaultTransferBrief(doc: ColliderExperimentDoc, index: number): string {
  const combination = doc.result.combinations[index];
  if (!combination) return '';
  return `${combination.name}: ${combination.description}\n\n${doc.result.productization.conceptBrief}`;
}

export function ColliderTransferPanel({
  client,
  workspaceId,
  projectId,
  experiment,
  experimentArtifact,
  notify,
  onTransferred,
  busy,
}: {
  client: Client;
  workspaceId: string;
  projectId: string;
  experiment: ColliderExperimentDoc;
  experimentArtifact: ArtifactRecord;
  notify: (message: string, isError: boolean) => void;
  onTransferred: (artifactId: string) => void;
  busy: boolean;
}) {
  const [selected, setSelected] = useState(0);
  const [brief, setBrief] = useState(() => defaultTransferBrief(experiment, 0));
  const [transferring, setTransferring] = useState(false);
  const [transferredId, setTransferredId] = useState('');
  // The draft a receipt belongs to. Editing the brief
  // or changing the selection clears the receipt, so
  // an old concept is never displayed as approval of
  // a new draft; a late response is applied only when
  // the draft still equals the submitted snapshot.
  const draftRef = useRef({ selected: 0, brief: defaultTransferBrief(experiment, 0) });

  function select(index: number) {
    setSelected(index);
    const next = defaultTransferBrief(experiment, index);
    setBrief(next);
    draftRef.current = { selected: index, brief: next };
    setTransferredId('');
  }

  function editBrief(next: string) {
    setBrief(next);
    draftRef.current = { selected: draftRef.current.selected, brief: next };
    setTransferredId('');
  }

  async function approve() {
    if (brief.trim() === '') return;
    // The brief is submitted exactly as edited:
    // validation rejects whitespace-only input, but
    // accepted indentation, tabs and terminal
    // newlines are recorded verbatim.
    const submitted = { selected, brief };
    setTransferring(true);
    try {
      const transferred = await client.transferProjectCollider(workspaceId, {
        projectId,
        experimentArtifactId: experimentArtifact.artifactId,
        experimentVersion: experimentArtifact.version,
        combinationIndex: selected,
        approvedBrief: brief,
      });
      if (
        draftRef.current.selected === submitted.selected
        && draftRef.current.brief === submitted.brief
      ) {
        setTransferredId(transferred.artifact.artifactId);
        notify(
          transferred.alreadyTransferred
            ? 'This transfer already exists; no duplicate was created.'
            : `Concept sent to Product Forge as artifact ${transferred.artifact.artifactId}.`,
          false,
        );
      } else {
        notify(
          'Transfer completed for the previous draft; the current draft has not been approved.',
          false,
        );
      }
    } catch (e) {
      notify(e instanceof Error ? e.message : 'Transfer failed.', true);
    } finally {
      setTransferring(false);
    }
  }

  const combination = experiment.result.combinations[selected];
  if (!combination) return null;
  return (
    <div>
      <h4>Send a combination to Product Forge</h4>
      <p className="help">
        Select one proposed combination, review the preview and edit the brief
        as you like. Opening this panel, selecting or editing saves nothing and
        contacts no provider. Only "Approve and send to Product Forge" creates
        the concept. Transfer approval is not specification approval: it does
        not approve a specification, authorize paid inference or run code.
        The transfer stays in this project.
      </p>
      <fieldset>
        <legend>Proposed combination to transfer</legend>
        {experiment.result.combinations.map((item, i) => (
          <label key={i}>
            <input
              type="radio"
              name="collider-transfer-combination"
              checked={selected === i}
              onChange={() => select(i)}
            />{' '}
            {item.name}: {item.description}
          </label>
        ))}
      </fieldset>
      <p>
        <strong>Source.</strong> Collider experiment {experimentArtifact.artifactId}{' '}
        version {experimentArtifact.version}, provider {experiment.provider}, model{' '}
        {experiment.model}.
      </p>
      <p>
        <strong>Selected result (model-derived, verbatim).</strong>{' '}
        {combination.name}: {combination.description}
      </p>
      <label htmlFor="collider-transfer-brief">
        Proposed concept brief — edit before approving (your text is recorded
        as user-authored)
      </label>
      <textarea
        id="collider-transfer-brief"
        value={brief}
        onChange={e => editBrief(e.target.value)}
        required
        maxLength={4000}
        rows={6}
      />
      <p>
        <strong>Assumptions (model-derived, verbatim).</strong>
      </p>
      <ul>
        {experiment.result.feasibility.hypotheses.map((item, i) => (
          <li key={i}>{item}</li>
        ))}
      </ul>
      <p>
        <strong>Unknowns (model-derived, verbatim).</strong>{' '}
        {[...experiment.result.feasibility.unknownCosts, ...experiment.result.feasibility.dependencies].join('; ') || 'None stated.'}
      </p>
      {transferredId ? (
        <div>
          <p role="status">
            Concept sent to Product Forge as artifact {transferredId}.
          </p>
          <button type="button" onClick={() => onTransferred(transferredId)}>
            Open in Product Forge
          </button>
        </div>
      ) : (
        <button
          type="button"
          disabled={busy || transferring || brief.trim() === ''}
          onClick={() => void approve()}
        >
          Approve and send to Product Forge
        </button>
      )}
    </div>
  );
}
