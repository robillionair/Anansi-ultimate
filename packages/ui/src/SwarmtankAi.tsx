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
  if (!exactKeys(value, ['round', 'actions', 'eventsDue'])) return null;
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
  onClose,
}: {
  doc: SwarmtankAiRunDoc;
  onClose?: () => void;
}) {
  const agentById = new Map(doc.agents.map(a => [a.id, a]));
  const totalActions = doc.rounds.reduce((n, r) => n + r.actions.length, 0);
  const failedActions = doc.rounds.reduce(
    (n, r) => n + r.actions.filter(a => a.status === 'failed').length,
    0,
  );
  return (
    <section aria-label="SwarmTank AI run">
      <h3>SwarmTank AI run — {statusLabel(doc.status)}</h3>
      <p>
        Conditional simulation result. Model {doc.modelProvider}/{doc.model} ({doc.locality}).
        {' '}{doc.callsMade} provider calls, {doc.tokensUsed} tokens used.
        {' '}{totalActions} agent actions ({failedActions} failed).
        {doc.market?.territoryName ? ` Market: ${doc.market.territoryName}.` : ''}
      </p>
      {doc.assumptions.map(assumption => (
        <p key={assumption}>{assumption}</p>
      ))}
      {doc.rounds.map(round => (
        <section key={round.round} aria-label={`AI round ${round.round}`}>
          <h4>Round {round.round}</h4>
          {round.eventsDue.length > 0 ? (
            <p>Events due: {round.eventsDue.join(', ')}.</p>
          ) : null}
          <ul>
            {round.actions.map((action, index) => {
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
      ))}
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
}) {
  const [agents, setAgents] = useState(3);
  const [rounds, setRounds] = useState(2);
  const [maxOutputTokens, setMaxOutputTokens] = useState(512);
  const [maxTotalTokens, setMaxTotalTokens] = useState(20000);
  const [estimate, setEstimate] = useState<SwarmtankAiEstimateResponse | null>(null);
  const [estimateError, setEstimateError] = useState<string | null>(null);
  const [runId, setRunId] = useState<string | null>(null);
  const [poll, setPoll] = useState<SwarmtankAiPollResponse | null>(null);
  const [polling, setPolling] = useState(false);
  const [openedRun, setOpenedRun] = useState<SwarmtankAiRunDoc | null>(null);
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
  }, [client, workspaceId, projectId, input, market, seed, agents, rounds, maxOutputTokens, maxTotalTokens, inputReady]);

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
      </label>
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
        <SwarmtankAiRunView doc={openedRun} onClose={() => setOpenedRun(null)} />
      ) : null}
    </section>
  );
}
