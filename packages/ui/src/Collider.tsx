import { useEffect, useRef, useState } from 'react';
import type {
  Client,
  ColliderControls,
  ColliderExperimentDoc,
  ColliderResult,
  DisclosureRecord,
} from '@anansi/contracts';
import { DisclosurePanel } from './Providers';
import { ColliderRunController } from './colliderRun';

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
  const trimmed = value.trim();
  if (trimmed.length === 0 || trimmed.length > max) return null;
  return trimmed;
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
  if (
    !exactKeys(parsed, [
      'format',
      'version',
      'beamA',
      'beamB',
      'controls',
      'result',
      'provider',
      'model',
      'disclosure',
    ])
  ) {
    return null;
  }
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
  return parsed as unknown as ColliderExperimentDoc;
}

// ---------------------------------------------------------------------------
// Result rendering

export function ColliderResultView({ result }: { result: ColliderResult }) {
  return (
    <div>
      <p className="help">
        All six sections below are model-generated hypotheses, analogies and
        assumptions — not researched facts. Nothing here establishes novelty,
        patentability, technical validation or market evidence.
      </p>
      <section aria-labelledby="collider-understanding-heading">
        <h4 id="collider-understanding-heading">Source understanding</h4>
        <p>{result.sourceUnderstanding}</p>
      </section>
      <section aria-labelledby="collider-primitives-heading">
        <h4 id="collider-primitives-heading">Conceptual primitives</h4>
        <ul>
          {result.conceptualPrimitives.map((primitive, i) => (
            <li key={i}>{primitive}</li>
          ))}
        </ul>
      </section>
      <section aria-labelledby="collider-mappings-heading">
        <h4 id="collider-mappings-heading">Cross-domain mappings</h4>
        <ul>
          {result.crossDomainMappings.map((mapping, i) => (
            <li key={i}>{mapping}</li>
          ))}
        </ul>
      </section>
      <section aria-labelledby="collider-combinations-heading">
        <h4 id="collider-combinations-heading">
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
      <section aria-labelledby="collider-feasibility-heading">
        <h4 id="collider-feasibility-heading">
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
      <section aria-labelledby="collider-productization-heading">
        <h4 id="collider-productization-heading">Productization proposals</h4>
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

export function ColliderExperimentView({ doc }: { doc: ColliderExperimentDoc }) {
  return (
    <div>
      <p>
        Collider experiment — provider {doc.provider}, model {doc.model}.
        Beams, controls and result below are exactly as saved; this record is
        immutable. It was structurally validated when generated, not factually
        verified.
      </p>
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
        <pre>{doc.beamA}</pre>
      </details>
      <details>
        <summary>Beam B (as supplied)</summary>
        <pre>{doc.beamB}</pre>
      </details>
      <ColliderResultView result={doc.result} />
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

export function ColliderPanel({
  client,
  workspaceId,
  projectId,
  notify,
  onSaved,
  busy,
}: {
  client: Client;
  workspaceId: string;
  projectId: string;
  notify: (message: string, isError: boolean) => void;
  onSaved: () => void;
  busy: boolean;
}) {
  const [beamA, setBeamA] = useState('');
  const [beamB, setBeamB] = useState('');
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

  useEffect(
    () => () => {
      void controllerRef.current?.reset();
    },
    [],
  );

  async function collide() {
    setResult(null);
    setSavedArtifact('');
    setValidating(false);
    const started = await controller().start({
      projectId,
      beamA,
      beamB,
      controls,
      zeroCostLock: lock,
    });
    if (!started) return; // superseded while the start was in flight
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
        Later slices will add URL fetching, document ingestion, saved-project
        and memory selection, 3D visualization and Forge transfer; none of
        those are available here.
      </p>
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
          onChange={e => setBeamA(e.target.value)}
          required
          maxLength={8000}
          rows={5}
        />
        <label htmlFor="collider-beam-b">Beam B</label>
        <textarea
          id="collider-beam-b"
          value={beamB}
          onChange={e => setBeamB(e.target.value)}
          required
          maxLength={8000}
          rows={5}
        />
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
        <button type="submit" disabled={busy || running}>
          Collide Ideas
        </button>
        {run?.status === 'running' && (
          <button type="button" className="secondary" onClick={() => void cancelRun()}>
            Cancel collision
          </button>
        )}
      </form>
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
      {disclosure && <DisclosurePanel disclosure={disclosure} />}
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
