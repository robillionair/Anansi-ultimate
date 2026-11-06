// ---------------------------------------------------------------------------
// SwarmTank v2 Checkpoint 2: AI simulation mode (UI).
//
// Quick simulation (seeded, deterministic, no provider call) stays the
// default. AI simulation runs distinct synthetic agents round by round
// through the configured provider, with the estimated call budget shown
// before Start, live progress while the run is in flight, and every
// failed provider call and malformed model output visible in the run
// history. A completed (or cancelled, or budget-exhausted) run is saved
// as a `swarmtank-ai-run` artifact and reopens from that artifact with
// no provider call. Outcomes are labelled conditional simulation
// results wherever they are shown.
// ---------------------------------------------------------------------------

import { useEffect, useRef, useState } from 'react';
import { SwarmtankAiMatrix } from './larp-mode/SwarmtankAiMatrix.tsx';
import { isReducedMotion } from './swarmtankThreeScene.ts';
import type {
  AiAgent,
  AiRound,
  ArtifactRecord,
  Client,
  SwarmtankAiEstimateResponse,
  SwarmtankAiPollResponse,
  SwarmtankAiRunDoc,
  SwarmtankInput,
  SwarmtankMarket,
} from '@anansi/contracts';

const AI_ACTIONS = ['request_evidence', 'defer', 'recommend', 'reject'] as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, expected: string[]): boolean {
  const keys = Object.keys(value);
  return (
    keys.length === expected.length && expected.every(key => keys.includes(key))
  );
}

function textField(value: unknown, max: number, required: boolean): string | null {
  if (typeof value !== 'string') return null;
  if (required && value.trim() === '') return null;
  if (value.length > max) return null;
  return value;
}

function parseAiAgent(value: unknown): AiAgent | null {
  if (!isRecord(value)) return null;
  const allowed = ['id', 'personaId', 'name', 'role', 'goal', 'constraints', 'language'];
  const keys = Object.keys(value);
  if (!keys.every(k => allowed.includes(k))) return null;
  for (const key of ['id', 'personaId', 'name', 'role', 'goal']) {
    if (textField(value[key], 500, true) === null) return null;
  }
  if (!Array.isArray(value.constraints)) return null;
  for (const item of value.constraints) {
    if (typeof item !== 'string' || item.length > 2000) return null;
  }
  if (value.language !== undefined && textField(value.language, 10, false) === null) {
    return null;
  }
  return value as unknown as AiAgent;
}

function parseAiRound(value: unknown): AiRound | null {
  if (!isRecord(value)) return null;
  if (!exactKeys(value, ['round', 'actions', 'eventsDue', 'date']) && !exactKeys(value, ['round', 'actions', 'eventsDue'])) return null;
  if (typeof value.round !== 'number' || value.round < 1) return null;
  if (!Array.isArray(value.actions) || !Array.isArray(value.eventsDue)) return null;
  for (const action of value.actions) {
    if (!isRecord(action)) return null;
    const allowed = [
      'agentId', 'round', 'action', 'rationale', 'evidenceRef',
      'priorInteractionRef', 'status', 'failure', 'rawOutput', 'model',
      'usage', 'memory',
    ];
    if (!Object.keys(action).every(k => allowed.includes(k))) return null;
    if (textField(action.agentId, 60, true) === null) return null;
    if (typeof action.round !== 'number' || action.round < 1) return null;
    if (action.action !== undefined && typeof action.action !== 'string') return null;
    if (action.action !== undefined && !AI_ACTIONS.includes(action.action as (typeof AI_ACTIONS)[number])) return null;
    if (action.status !== 'ok' && action.status !== 'failed') return null;
  }
  return value as unknown as AiRound;
}

/**
 * Strict client-side parsing for a saved AI run body. The content hash
 * is verified server-side on restore; this parser checks the shape,
 * the format/version, the run status, and that every recorded action
 * is one of the four validated actions (or a recorded failure).
 */
export function parseSwarmtankAiRunBody(raw: string): SwarmtankAiRunDoc | null {
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!isRecord(value)) return null;
  const allowed = [
    'format', 'version', 'runId', 'workspaceId', 'projectId', 'input',
    'market', 'seed', 'agents', 'scheduledEvents', 'rounds', 'status',
    'modelProvider', 'model', 'locality', 'caps', 'callsMade', 'tokensUsed',
    'assumptions', 'contentHash', 'createdMs',
    'startDate', 'endDate', 'stepDays', 'stepCoarsened',
    'parentRunId', 'parentArtifactId', 'parentRound', 'branchChanges',
  ];
  if (!Object.keys(value).every(k => allowed.includes(k))) return null;
  if (value.format !== 'anansi-swarmtank-ai-run') return null;
  if (value.version !== 1) return null;
  if (textField(value.runId, 120, true) === null) return null;
  if (!['complete', 'cancelled', 'budget-exhausted', 'failed'].includes(value.status as string)) return null;
  if (!Array.isArray(value.agents) || !Array.isArray(value.rounds)) return null;
  for (const agent of value.agents) {
    if (parseAiAgent(agent) === null) return null;
  }
  for (const round of value.rounds) {
    if (parseAiRound(round) === null) return null;
  }
  if (typeof value.contentHash !== 'string' || value.contentHash.length !== 64) return null;
  if (typeof value.startDate !== 'string' || typeof value.stepDays !== 'number') return null;
  if (typeof value.stepCoarsened !== 'boolean') return null;
  return value as unknown as SwarmtankAiRunDoc;
}

function actionLabel(action: string | null | undefined): string {
  switch (action) {
    case 'recommend':
      return 'Recommend';
    case 'reject':
      return 'Reject';
    case 'defer':
      return 'Defer';
    case 'request_evidence':
      return 'Request evidence';
    default:
      return 'No valid action';
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'complete':
      return 'Complete';
    case 'cancelled':
      return 'Cancelled';
    case 'budget-exhausted':
      return 'Budget exhausted';
    case 'failed':
      return 'Failed';
    case 'running':
      return 'Running';
    case 'cancelling':
      return 'Cancelling';
    default:
      return status;
  }
}

export function SwarmtankAiRunView({
  doc,
  artifactId,
  client,
  workspaceId,
  onBranchStarted,
  onClose,
}: {
  doc: SwarmtankAiRunDoc;
  artifactId?: string;
  client?: Client;
  workspaceId?: string;
  onBranchStarted?: (runId: string) => void;
  onClose?: () => void;
}) {
  const agentById = new Map(doc.agents.map(a => [a.id, a]));
  const totalActions = doc.rounds.reduce((n, r) => n + r.actions.length, 0);
  const failedActions = doc.rounds.reduce(
    (n, r) => n + r.actions.filter(a => a.status === 'failed').length,
    0,
  );
  const [selectedRound, setSelectedRound] = useState(doc.rounds.length);
  const selected = doc.rounds.find(r => r.round === selectedRound) ?? doc.rounds[doc.rounds.length - 1];
  const [branchRound, setBranchRound] = useState(doc.rounds.length > 1 ? doc.rounds.length - 1 : 1);
  const [branchPrice, setBranchPrice] = useState('');
  const [branchEventKind, setBranchEventKind] = useState('price_change');
  const [branchEventDescription, setBranchEventDescription] = useState('');
  const [branchEventRound, setBranchEventRound] = useState(doc.rounds.length);
  const [branchError, setBranchError] = useState<string | null>(null);
  const [branchBusy, setBranchBusy] = useState(false);
  const [comparison, setComparison] = useState<SwarmtankAiRunDoc | null>(null);
  const [comparisonError, setComparisonError] = useState<string | null>(null);
  const isBranch = doc.parentRunId !== undefined && doc.parentRunId !== null;
  const horizonDays =
    doc.endDate && doc.startDate
      ? Math.round(
          (Date.parse(doc.endDate) - Date.parse(doc.startDate)) / 86400000,
        )
      : 0;

  async function startBranch() {
    if (!client || !workspaceId) return;
    setBranchError(null);
    setBranchBusy(true);
    try {
      const price = branchPrice.trim() === '' ? undefined : Number(branchPrice);
      if (price !== undefined && (!Number.isFinite(price) || price < 0)) {
        setBranchError('The branch price must be a number of 0 or more.');
        return;
      }
      const newEvents =
        branchEventDescription.trim() === ''
          ? undefined
          : [
              {
                round: branchEventRound,
                kind: branchEventKind,
                description: branchEventDescription.trim(),
              },
            ];
      if (price === undefined && newEvents === undefined) {
        setBranchError('Change the price or add an event to branch from this round.');
        return;
      }
      const result = await client.branchSwarmtankAi(workspaceId, {
        projectId: doc.projectId,
        parentArtifactId: artifactId ?? '',
        parentRound: branchRound,
        priceAmount: price,
        newEvents,
        rounds: doc.rounds.length,
      });
      // Track the branch run's progress in the panel (the poll loop
      // reads runIdRef, so set both the ref and the state).
      if (onBranchStarted) onBranchStarted(result.runId);
    } catch (err) {
      setBranchError(err instanceof Error ? err.message : 'Could not start the branch.');
    } finally {
      setBranchBusy(false);
    }
  }

  async function loadComparison() {
    if (!client || !workspaceId || !doc.parentArtifactId) return;
    setComparisonError(null);
    try {
      const details = await client.readProjectArtifact(workspaceId, doc.parentArtifactId, undefined);
      const parent = parseSwarmtankAiRunBody(details.body);
      if (parent === null) {
        setComparisonError('The parent run body is not readable.');
        return;
      }
      setComparison(parent);
    } catch (err) {
      setComparisonError(err instanceof Error ? err.message : 'Could not load the parent run.');
    }
  }

  return (
    <section aria-label="SwarmTank AI run">
      <h3>SwarmTank AI run — {statusLabel(doc.status)}</h3>
      <p>
        Conditional simulation result. Model {doc.modelProvider}/{doc.model} ({doc.locality}).
        {' '}{doc.callsMade} provider calls, {doc.tokensUsed} tokens used.
        {' '}{totalActions} agent actions ({failedActions} failed).
        {doc.market?.territoryName ? ` Market: ${doc.market.territoryName}.` : ''}
        {isBranch ? ` Branch of run ${doc.parentRunId} from round ${doc.parentRound}.` : ''}
        {doc.endDate ? ` Horizon: ${doc.startDate} to ${doc.endDate} (${horizonDays} days, ${doc.stepDays}-day steps${doc.stepCoarsened ? ', step coarsened to fit the run budget' : ''}).` : ''}
      </p>
      {doc.endDate && horizonDays >= 90 ? (
        <p>
          <strong>Long horizon — assumptions matter more.</strong> The further
          out the simulation runs, the more its outcomes depend on the
          assumptions below rather than on the starting evidence.
        </p>
      ) : null}
      {doc.assumptions.map(assumption => (
        <p key={assumption}>{assumption}</p>
      ))}
      <section aria-label="AI timeline">
        <h4>Timeline</h4>
        <label>
          Round{' '}
          <input
            aria-label="AI timeline round"
            type="range"
            min={1}
            max={Math.max(1, doc.rounds.length)}
            value={selectedRound}
            onChange={event => setSelectedRound(Number(event.target.value))}
          />
          {selected ? `Round ${selected.round} of ${doc.rounds.length}${selected.date ? ` — ${selected.date}` : ''}` : ''}
        </label>
        <div>
          {doc.rounds.map(round => (
            <button
              key={round.round}
              type="button"
              aria-pressed={selectedRound === round.round}
              onClick={() => setSelectedRound(round.round)}
            >
              Round {round.round}
              {round.date ? ` (${round.date})` : ''}
            </button>
          ))}
        </div>
      </section>
      {selected ? (
        <section aria-label={`AI round ${selected.round} detail`}>
          <h4>
            Round {selected.round}
            {selected.date ? ` — ${selected.date}` : ''}
          </h4>
          {selected.eventsDue.length > 0 ? (
            <p>Events due: {selected.eventsDue.join(', ')}.</p>
          ) : null}
          <ul>
            {selected.actions.map((action, index) => {
              const agent = agentById.get(action.agentId);
              return (
                <li key={`${action.agentId}-${index}`}>
                  <strong>{agent ? agent.name : action.agentId}</strong>
                  {agent ? ` — ${agent.role}. ` : ' '}
                  {actionLabel(action.action)}.
                  {action.rationale ? ` ${action.rationale}` : ''}
                  {action.evidenceRef ? ` Cited: ${action.evidenceRef}.` : ''}
                  {action.priorInteractionRef ? ` Prior: ${action.priorInteractionRef}.` : ''}
                  {action.model ? ` (${action.model})` : ''}
                  {action.usage?.totalTokens !== undefined
                    ? ` ${action.usage.totalTokens} tokens.`
                    : ''}
                  {action.status === 'failed' ? (
                    <>
                      {' '}
                      <strong>Failed.</strong>
                      {action.failure ? ` ${action.failure}` : ''}
                      {action.rawOutput ? ` Raw output: ${action.rawOutput}` : ''}
                    </>
                  ) : null}
                </li>
              );
            })}
          </ul>
        </section>
      ) : null}
      {client && workspaceId && doc.rounds.length > 0 ? (
        <section aria-label="Branch from this run">
          <h4>Branch from this run (what-if)</h4>
          <p>
            Continue from a saved round with a changed price or a new event.
            The branch keeps the original rounds up to the branch point and
            runs new rounds after it. Both timelines are conditional
            simulation results.
          </p>
          <label>
            Branch from round{' '}
            <select
              aria-label="Branch from round"
              value={branchRound}
              onChange={event => setBranchRound(Number(event.target.value))}
            >
              {doc.rounds.map(round => (
                <option key={round.round} value={round.round}>
                  Round {round.round}
                  {round.date ? ` (${round.date})` : ''}
                </option>
              ))}
            </select>
          </label>{' '}
          <label>
            New price amount (optional){' '}
            <input
              aria-label="Branch price amount"
              type="number"
              value={branchPrice}
              onChange={event => setBranchPrice(event.target.value)}
              placeholder="e.g. 79"
            />
          </label>{' '}
          <label>
            New event kind{' '}
            <select
              aria-label="Branch event kind"
              value={branchEventKind}
              onChange={event => setBranchEventKind(event.target.value)}
            >
              <option value="price_change">Price change</option>
              <option value="competitor_launch">Competitor launch</option>
              <option value="scenario">Scenario event</option>
            </select>
          </label>{' '}
          <label>
            New event description (optional){' '}
            <input
              aria-label="Branch event description"
              value={branchEventDescription}
              onChange={event => setBranchEventDescription(event.target.value)}
              placeholder="e.g. A competitor launches a cheaper tier."
              maxLength={1000}
            />
          </label>{' '}
          <label>
            New event round{' '}
            <input
              aria-label="Branch event round"
              type="number"
              min={branchRound + 1}
              max={doc.rounds.length}
              value={branchEventRound}
              onChange={event => setBranchEventRound(Number(event.target.value))}
            />
          </label>
          {branchError !== null ? <p role="alert">{branchError}</p> : null}
          <p>
            <button type="button" disabled={branchBusy} onClick={startBranch}>
              Start branch
            </button>
          </p>
        </section>
      ) : null}
      {isBranch && client && workspaceId ? (
        <section aria-label="Branch comparison">
          <h4>Compare with the original timeline</h4>
          <p>
            Both timelines are conditional simulation results. The branch
            shares the original rounds up to round {doc.parentRound}; the
            timelines diverge after it.
          </p>
          {comparisonError !== null ? <p role="alert">{comparisonError}</p> : null}
          {comparison === null ? (
            <p>
              <button type="button" onClick={loadComparison}>
                Compare with original
              </button>
            </p>
          ) : (
            <SwarmtankAiComparison original={comparison} branch={doc} />
          )}
        </section>
      ) : null}
      {onClose ? (
        <p>
          <button type="button" onClick={onClose}>
            Close AI run
          </button>
        </p>
      ) : null}
    </section>
  );
}

export function SwarmtankAiComparison({
  original,
  branch,
}: {
  original: SwarmtankAiRunDoc;
  branch: SwarmtankAiRunDoc;
}) {
  const branchFrom = branch.parentRound ?? 0;
  const agentById = new Map(branch.agents.map(a => [a.id, a]));
  const originalByRound = new Map(original.rounds.map(r => [r.round, r]));
  return (
    <section aria-label="Branch comparison table">
      <p>
        Conditional simulation results — original timeline beside the branch.
        {branch.branchChanges?.priceAmount !== undefined
          ? ` Branch price: $${branch.branchChanges.priceAmount} per month (original: $${original.input.priceAmount ?? '—'}).`
          : ''}
      </p>
      {branch.rounds.map(round => {
        const orig = originalByRound.get(round.round);
        const shared = round.round <= branchFrom;
        return (
          <section key={round.round} aria-label={`Comparison round ${round.round}`}>
            <h5>
              Round {round.round}
              {round.date ? ` — ${round.date}` : ''}
              {shared ? ' (shared — identical to the original)' : ' (diverged)'}
            </h5>
            <div>
              <p>
                <strong>Original:</strong>{' '}
                {orig
                  ? orig.actions.map(a => `${a.agentId}: ${actionLabel(a.action)}`).join('; ')
                  : '—'}
              </p>
              <p>
                <strong>Branch:</strong>{' '}
                {round.actions.map(a => `${a.agentId}: ${actionLabel(a.action)}`).join('; ')}
              </p>
            </div>
            {round.actions.map((action, index) => {
              const agent = agentById.get(action.agentId);
              return (
                <p key={`${action.agentId}-${index}`}>
                  {agent ? agent.name : action.agentId}: {actionLabel(action.action)}.
                  {action.rationale ? ` ${action.rationale}` : ''}
                </p>
              );
            })}
          </section>
        );
      })}
    </section>
  );
}

export function SwarmtankAiPanel({
  client,
  workspaceId,
  projectId,
  input,
  market,
  seed,
  artifacts,
  onSaved,
  notify,
  busy,
  cinematic = false,
}: {
  client: Client;
  workspaceId: string;
  projectId: string;
  input: SwarmtankInput;
  market: SwarmtankMarket | null;
  seed: string;
  artifacts: ArtifactRecord[];
  onSaved: () => Promise<void>;
  notify: (message: string, isError: boolean) => void;
  busy: boolean;
  cinematic?: boolean;
}) {
  const [agents, setAgents] = useState(3);
  const [rounds, setRounds] = useState(2);
  const [maxOutputTokens, setMaxOutputTokens] = useState(512);
  const [maxTotalTokens, setMaxTotalTokens] = useState(20000);
  const [endDate, setEndDate] = useState('');
  const [stepDays, setStepDays] = useState(30);
  const [events, setEvents] = useState<{ round: number; kind: string; description: string }[]>([]);
  const [eventKind, setEventKind] = useState('price_change');
  const [eventDescription, setEventDescription] = useState('');
  const [eventRound, setEventRound] = useState(2);
  const [estimate, setEstimate] = useState<SwarmtankAiEstimateResponse | null>(null);
  const [estimateError, setEstimateError] = useState<string | null>(null);
  const [runId, setRunId] = useState<string | null>(null);
  const [poll, setPoll] = useState<SwarmtankAiPollResponse | null>(null);
  const [polling, setPolling] = useState(false);
  const [openedRun, setOpenedRun] = useState<SwarmtankAiRunDoc | null>(null);
  const [reduceMotion, setReduceMotion] = useState(() => isReducedMotion());
  const [openedArtifactId, setOpenedArtifactId] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const runIdRef = useRef<string | null>(null);
  const pollRef = useRef<SwarmtankAiPollResponse | null>(null);

  const aiRuns = artifacts.filter(a => a.type === 'swarmtank-ai-run');
  const inputReady = input.description.trim() !== '';

  // The estimated call budget, before Start: mode, provider + model,
  // locality, and the estimated calls and tokens. No provider call.
  useEffect(() => {
    if (!inputReady) {
      setEstimate(null);
      setEstimateError(null);
      return;
    }
    let active = true;
    const timer = setTimeout(() => {
      void client
        .estimateSwarmtankAi(workspaceId, {
          projectId,
          input,
          market: market ?? undefined,
          seed: seed.trim() === '' ? undefined : seed,
          agents,
          rounds,
          maxOutputTokens,
          maxTotalTokens,
          endDate: endDate.trim() === '' ? undefined : endDate.trim(),
          stepDays,
          scheduledEvents: events.length > 0 ? events : undefined,
        })
        .then(result => {
          if (active) {
            setEstimate(result);
            setEstimateError(null);
          }
        })
        .catch(err => {
          if (active) {
            setEstimate(null);
            setEstimateError(err instanceof Error ? err.message : 'Could not estimate the AI run.');
          }
        });
    }, 400);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [client, workspaceId, projectId, input, market, seed, agents, rounds, maxOutputTokens, maxTotalTokens, endDate, stepDays, events, inputReady]);

  // Poll while a run is in flight. The interval reads the run id from
  // a ref so the closure never goes stale.
  useEffect(() => {
    if (runId === null || !polling) return;
    const timer = setInterval(() => {
      const id = runIdRef.current;
      if (id === null) return;
      void client
        .pollSwarmtankAi(workspaceId, id)
        .then(result => {
          pollRef.current = result;
          setPoll(result);
          if (result.status !== 'running' && result.status !== 'cancelling') {
            setPolling(false);
            void onSaved();
            if (result.status === 'complete') {
              notify('AI simulation complete and saved.', false);
            } else if (result.status === 'budget-exhausted') {
              notify('AI simulation stopped: the token budget was exhausted. The run is saved with the rounds completed so far.', false);
            } else if (result.status === 'cancelled') {
              notify('AI simulation cancelled. The run is saved with the rounds completed so far.', false);
            } else if (result.status === 'failed') {
              notify(result.error ?? 'AI simulation failed.', true);
            }
          }
        })
        .catch(err => {
          setPolling(false);
          notify(err instanceof Error ? err.message : 'Could not poll the AI run.', true);
        });
    }, 2000);
    return () => clearInterval(timer);
  }, [client, workspaceId, runId, polling, onSaved, notify]);

  async function start() {
    if (!inputReady) return;
    setRunning(true);
    try {
      const result = await client.startSwarmtankAi(workspaceId, {
        projectId,
        input,
        market: market ?? undefined,
        seed: seed.trim() === '' ? undefined : seed,
        agents,
        rounds,
        maxOutputTokens,
        maxTotalTokens,
        endDate: endDate.trim() === '' ? undefined : endDate.trim(),
        stepDays,
        scheduledEvents: events.length > 0 ? events : undefined,
      });
      setRunId(result.runId);
      runIdRef.current = result.runId;
      setPoll(null);
      pollRef.current = null;
      setOpenedRun(null);
      setPolling(true);
      notify(
        `AI simulation started: ${result.provider}/${result.model} (${result.locality}), ${result.estimatedCalls} calls estimated, ${result.estimatedTokens} tokens estimated.`,
        false,
      );
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Could not start the AI simulation.', true);
    } finally {
      setRunning(false);
    }
  }

  async function cancel() {
    const id = runIdRef.current;
    if (id === null) return;
    try {
      await client.cancelSwarmtankAi(workspaceId, id);
      notify('Cancelling the AI simulation…', false);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Could not cancel the AI run.', true);
    }
  }

  async function openRun(artifact: ArtifactRecord) {
    try {
      const details = await client.readProjectArtifact(workspaceId, artifact.artifactId, undefined);
      const doc = parseSwarmtankAiRunBody(details.body);
      if (doc === null) {
        notify('That AI run body is not readable.', true);
        return;
      }
      setOpenedRun(doc);
      setOpenedArtifactId(artifact.artifactId);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Could not open the AI run.', true);
    }
  }

  const live = poll !== null && (poll.status === 'running' || poll.status === 'cancelling');

  return (
    <section aria-label="AI simulation">
      <h3>AI simulation</h3>
      <p>
        Distinct synthetic agents, each with its own goal and constraints,
        answer round by round through the configured provider. Every agent
        is simulated; outcomes are conditional simulation results, not
        measured demand. A failed provider call or a malformed model output
        is recorded in the run history — the run continues with the
        remaining agents.
      </p>
      <label>
        Agents{' '}
        <input
          aria-label="AI agents"
          type="number"
          min={1}
          max={10}
          value={agents}
          onChange={event => setAgents(Number(event.target.value))}
        />
      </label>{' '}
      <label>
        Rounds{' '}
        <input
          aria-label="AI rounds"
          type="number"
          min={1}
          max={10}
          value={rounds}
          onChange={event => setRounds(Number(event.target.value))}
        />
      </label>{' '}
      <label>
        Max output tokens per call{' '}
        <input
          aria-label="AI max output tokens"
          type="number"
          min={64}
          max={2048}
          value={maxOutputTokens}
          onChange={event => setMaxOutputTokens(Number(event.target.value))}
        />
      </label>{' '}
      <label>
        Max total tokens for the run{' '}
        <input
          aria-label="AI max total tokens"
          type="number"
          min={1000}
          max={200000}
          value={maxTotalTokens}
          onChange={event => setMaxTotalTokens(Number(event.target.value))}
        />
      </label>{' '}
      <label>
        Future end date (optional, YYYY-MM-DD){' '}
        <input
          aria-label="AI end date"
          type="date"
          value={endDate}
          onChange={event => setEndDate(event.target.value)}
        />
      </label>{' '}
      <label>
        Step size{' '}
        <select
          aria-label="AI step size"
          value={stepDays}
          onChange={event => setStepDays(Number(event.target.value))}
        >
          <option value={1}>Daily</option>
          <option value={7}>Weekly</option>
          <option value={30}>Monthly</option>
          <option value={90}>Quarterly</option>
        </select>
      </label>
      {estimate !== null && endDate.trim() !== '' ? (
        <p>
          Time plan: {estimate.stepCount} step{estimate.stepCount === 1 ? '' : 's'} of{' '}
          {estimate.effectiveStepDays} day{estimate.effectiveStepDays === 1 ? '' : 's'}.
          {estimate.stepCoarsened
            ? ' The horizon forced a coarser step than requested, so the run uses fewer, longer steps.'
            : ''}
        </p>
      ) : null}
      <section aria-label="Scheduled events">
        <h4>Scheduled events</h4>
        <p>
          Events the agents observe in the round they fall due — a price
          change or a competitor launch. They are part of the run record.
        </p>
        {events.length > 0 ? (
          <ul>
            {events.map((event, index) => (
              <li key={index}>
                Round {event.round}: {event.kind} — {event.description}{' '}
                <button
                  type="button"
                  onClick={() => setEvents(prev => prev.filter((_, i) => i !== index))}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        ) : null}
        <label>
          Event kind{' '}
          <select
            aria-label="Scheduled event kind"
            value={eventKind}
            onChange={event => setEventKind(event.target.value)}
          >
            <option value="price_change">Price change</option>
            <option value="competitor_launch">Competitor launch</option>
            <option value="scenario">Scenario event</option>
          </select>
        </label>{' '}
        <label>
          Event description{' '}
          <input
            aria-label="Scheduled event description"
            value={eventDescription}
            onChange={event => setEventDescription(event.target.value)}
            placeholder="e.g. A competitor launches a cheaper tier."
            maxLength={1000}
          />
        </label>{' '}
        <label>
          Event round{' '}
          <input
            aria-label="Scheduled event round"
            type="number"
            min={1}
            max={rounds}
            value={eventRound}
            onChange={event => setEventRound(Number(event.target.value))}
          />
        </label>{' '}
        <button
          type="button"
          disabled={eventDescription.trim() === ''}
          onClick={() => {
            setEvents(prev => [
              ...prev,
              { round: eventRound, kind: eventKind, description: eventDescription.trim() },
            ]);
            setEventDescription('');
          }}
        >
          Add event
        </button>
      </section>
      {estimateError !== null ? <p role="alert">{estimateError}</p> : null}
      {estimate !== null ? (
        <p>
          Estimated budget before Start: {estimate.provider}/{estimate.model} ({estimate.locality}),{' '}
          {estimate.estimatedCalls} provider calls, {estimate.estimatedTokens} tokens estimated.
          {estimate.locality === 'cloud'
            ? ' This model is cloud/paid: starting requires an explicit authorization (permit).'
            : ' Local model: no paid call.'}
        </p>
      ) : null}
      <p>
        <button disabled={running || busy || !inputReady || live} onClick={start}>
          Start AI simulation
        </button>{' '}
        {live ? (
          <button disabled={busy} onClick={cancel}>
            Cancel AI simulation
          </button>
        ) : null}
      </p>
      {poll !== null ? (
        <section aria-label="AI run progress">
          <h4>
            AI run — {statusLabel(poll.status)} ({poll.callsMade} calls, {poll.tokensUsed} tokens)
          </h4>
          {poll.agents.map(agent => (
            <p key={agent.id}>
              <strong>{agent.name}</strong> — {agent.role}.
            </p>
          ))}
          {poll.rounds.map(round => (
            <section key={round.round} aria-label={`AI round ${round.round} progress`}>
              <h5>Round {round.round}</h5>
              <ul>
                {round.actions.map((action, index) => (
                  <li key={`${action.agentId}-${index}`}>
                    <strong>{action.agentId}</strong>: {actionLabel(action.action)}.
                    {action.rationale ? ` ${action.rationale}` : ''}
                    {action.evidenceRef ? ` Cited: ${action.evidenceRef}.` : ''}
                    {action.status === 'failed' ? (
                      <>
                        {' '}
                        <strong>Failed.</strong>
                        {action.failure ? ` ${action.failure}` : ''}
                        {action.rawOutput ? ` Raw output: ${action.rawOutput}` : ''}
                      </>
                    ) : null}
                  </li>
                ))}
              </ul>
            </section>
          ))}
          {poll.status === 'budget-exhausted' ? (
            <p>
              The token budget was exhausted. The run stopped here and was saved with the rounds
              completed so far — this is a budget limit, not an error.
            </p>
          ) : null}
          {poll.artifactId ? (
            <p>
              Saved as AI run {poll.artifactId} (version {poll.artifactVersion}).
            </p>
          ) : null}
        </section>
      ) : null}
      {aiRuns.length > 0 ? (
        <section aria-label="Saved AI runs">
          <h3>Saved AI runs</h3>
          <ul>
            {aiRuns.map(artifact => (
              <li key={artifact.artifactId}>
                {artifact.title} (version {artifact.version}){' '}
                <button disabled={running || busy} onClick={() => openRun(artifact)}>
                  View AI run
                </button>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {openedRun !== null ? (
        <>
          <SwarmtankAiRunView
            doc={openedRun}
            artifactId={openedArtifactId ?? undefined}
            client={client}
            workspaceId={workspaceId}
            onBranchStarted={runId => {
              setRunId(runId);
              runIdRef.current = runId;
              setPoll(null);
              pollRef.current = null;
              setPolling(true);
              void onSaved();
              notify('Branch started. Watch the run progress above.', false);
            }}
            onClose={() => {
              setOpenedRun(null);
              setOpenedArtifactId(null);
            }}
          />
          {cinematic ? (
            <>
              <p>
                <label>
                  <input
                    type="checkbox"
                    aria-label="Reduce motion (static AI matrix view)"
                    checked={reduceMotion}
                    onChange={event => setReduceMotion(event.target.checked)}
                  />{' '}
                  Reduce motion (static AI matrix view)
                </label>
              </p>
              <SwarmtankAiMatrix key={openedRun.runId} doc={openedRun} staticView={reduceMotion} />
            </>
          ) : null}
        </>
      ) : null}
    </section>
  );
}
