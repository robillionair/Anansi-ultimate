import { useEffect, useRef, useState } from 'react';
import type {
  ArtifactRecord,
  Client,
  CompactionMetrics,
  CompactionPairResult,
  CompactionQuestion,
  SomniaCompactionCandidate,
  SomniaCompactionExperiment,
  SomniaCompactionPollResponse,
  SomniaCompactionPrepareResponse,
  SomniaCompactionRecord,
} from '@anansi/contracts';
import { SOMNIA_COMPACTION_BENCHMARK_CASES } from './somniaCompactionBenchmark';

const COMPACTION_EXPERIMENT_FORMAT = 'anansi-somnia-compaction-experiment';
const MAX_TOTAL_TOKENS_DEFAULT = 20_000;

// ---------------------------------------------------------------------------
// Pure helpers (exported for tests). These mirror the Rust implementations in
// crates/core/src/somnia_compaction_run.rs exactly: same abstention rule,
// same scoring, same metrics, and the same canonical JSON the server hashes.
// ---------------------------------------------------------------------------

export function isCompactionAbstention(reply: string): boolean {
  const trimmed = reply.trim();
  if (trimmed === '') return true;
  const upper = trimmed.toUpperCase();
  if (upper === 'UNKNOWN') return true;
  if (upper.startsWith('UNKNOWN ')) return true;
  return upper.length > 7 && upper.startsWith('UNKNOWN') && !/[A-Z0-9]/.test(upper[7]);
}

export function scoreCompactionReply(
  reply: string | null | undefined,
  expectedAnswer: string | null | undefined,
): 'correct' | 'incorrect' | 'abstained' | 'failed' {
  if (reply === null || reply === undefined) return 'failed';
  if (isCompactionAbstention(reply)) {
    return expectedAnswer === null || expectedAnswer === undefined ? 'correct' : 'abstained';
  }
  if (expectedAnswer === null || expectedAnswer === undefined) return 'incorrect';
  return reply.trim() === expectedAnswer.trim() ? 'correct' : 'incorrect';
}

export function computeCompactionMetrics(
  pairs: CompactionPairResult[],
  compactionMicrosTotal: number,
  totalLatencyMs: number,
): CompactionMetrics {
  let tokensOriginal: number | null = 0;
  let tokensCompacted: number | null = 0;
  let reportedOriginal = 0;
  let reportedCompacted = 0;
  const metrics: CompactionMetrics = {
    pairsTotal: pairs.length,
    pairsDone: pairs.length,
    correctOriginal: 0,
    correctCompacted: 0,
    abstentionsOriginal: 0,
    abstentionsCompacted: 0,
    failedPairs: 0,
    tokensOriginal: null,
    tokensCompacted: null,
    tokensReportedOriginalPairs: 0,
    tokensReportedCompactedPairs: 0,
    compactionMicrosTotal,
    totalLatencyMs,
  };
  for (const pair of pairs) {
    if (pair.originalScore === 'correct') metrics.correctOriginal += 1;
    if (pair.compactedScore === 'correct') metrics.correctCompacted += 1;
    if (pair.originalReply !== null && pair.originalReply !== undefined && isCompactionAbstention(pair.originalReply)) {
      metrics.abstentionsOriginal += 1;
    }
    if (pair.compactedReply !== null && pair.compactedReply !== undefined && isCompactionAbstention(pair.compactedReply)) {
      metrics.abstentionsCompacted += 1;
    }
    if (pair.originalScore === 'failed' || pair.compactedScore === 'failed') {
      metrics.failedPairs += 1;
    }
    const originalTotal = pair.originalUsage?.totalTokens ?? null;
    if (originalTotal === null || originalTotal === undefined) {
      tokensOriginal = null;
    } else if (tokensOriginal !== null) {
      tokensOriginal += originalTotal;
      reportedOriginal += 1;
    }
    const compactedTotal = pair.compactedUsage?.totalTokens ?? null;
    if (compactedTotal === null || compactedTotal === undefined) {
      tokensCompacted = null;
    } else if (tokensCompacted !== null) {
      tokensCompacted += compactedTotal;
      reportedCompacted += 1;
    }
  }
  metrics.tokensOriginal = tokensOriginal;
  metrics.tokensCompacted = tokensCompacted;
  metrics.tokensReportedOriginalPairs = reportedOriginal;
  metrics.tokensReportedCompactedPairs = reportedCompacted;
  return metrics;
}

function canonicalRecord(record: SomniaCompactionRecord): Record<string, unknown> {
  const out: Record<string, unknown> = {
    id: record.id,
    key: record.key,
    source: record.source,
    revision: record.revision,
    kind: record.kind,
    trust: record.trust,
    privacy: record.privacy,
    text: record.text,
  };
  // The Rust struct skips an empty `supersedes` on serialization, so the
  // canonical form omits it too. A non-empty list is kept verbatim.
  if (record.supersedes && record.supersedes.length > 0) {
    out.supersedes = record.supersedes;
  }
  return out;
}

function canonicalCandidate(candidate: SomniaCompactionCandidate): Record<string, unknown> {
  const retained = (list: { record: SomniaCompactionRecord; status: string; detail: string; conflictWith: string[] }[]) =>
    list.map(entry => ({
      record: canonicalRecord(entry.record),
      status: entry.status,
      detail: entry.detail,
      conflictWith: entry.conflictWith,
    }));
  return {
    format: candidate.format,
    version: candidate.version,
    inputHash: candidate.inputHash,
    retained: {
      trusted: retained(candidate.retained.trusted),
      unverified: retained(candidate.retained.unverified),
      conflicts: retained(candidate.retained.conflicts),
      deletions: retained(candidate.retained.deletions),
      uncertainties: retained(candidate.retained.uncertainties),
    },
    excluded: candidate.excluded.map(entry => ({
      record: canonicalRecord(entry.record),
      reason: entry.reason,
      detail: entry.detail,
    })),
    quarantined: candidate.quarantined.map(entry => ({
      recordRef: entry.recordRef,
      reason: entry.reason,
    })),
    stats: {
      inputRecords: candidate.stats.inputRecords,
      retainedRecords: candidate.stats.retainedRecords,
      excludedRecords: candidate.stats.excludedRecords,
      quarantinedRecords: candidate.stats.quarantinedRecords,
      inputChars: candidate.stats.inputChars,
      retainedChars: candidate.stats.retainedChars,
    },
    contentHash: candidate.contentHash,
  };
}

function canonicalPair(pair: CompactionPairResult): Record<string, unknown> {
  const usage = (value: CompactionPairResult['originalUsage']) => {
    if (value === null || value === undefined) return null;
    return {
      inputTokens: value.inputTokens ?? null,
      outputTokens: value.outputTokens ?? null,
      totalTokens: value.totalTokens ?? null,
    };
  };
  // The server omits the privacy fields when the arm is standard
  // (skip_serializing_if); the canonical form mirrors that, or the
  // experiment hash would disagree with the server.
  const out: Record<string, unknown> = {
    pairId: pair.pairId,
    caseId: pair.caseId,
    questionId: pair.questionId,
    question: pair.question,
    expectedAnswer: pair.expectedAnswer ?? null,
    originalPrompt: pair.originalPrompt,
    compactedPrompt: pair.compactedPrompt,
    originalReply: pair.originalReply ?? null,
    compactedReply: pair.compactedReply ?? null,
    originalScore: pair.originalScore,
    compactedScore: pair.compactedScore,
    originalUsage: usage(pair.originalUsage),
    compactedUsage: usage(pair.compactedUsage),
    failure: pair.failure ?? null,
    compactionMicros: pair.compactionMicros,
  };
  if (pair.originalPrivacy !== null && pair.originalPrivacy !== undefined) {
    out.originalPrivacy = pair.originalPrivacy;
  }
  if (pair.compactedPrivacy !== null && pair.compactedPrivacy !== undefined) {
    out.compactedPrivacy = pair.compactedPrivacy;
  }
  return out;
}

/** The canonical JSON the server hashes: Rust struct field order, hash emptied. */
export function canonicalCompactionExperimentJson(
  experiment: SomniaCompactionExperiment,
): string {
  return JSON.stringify({
    format: experiment.format,
    version: experiment.version,
    runId: experiment.runId,
    provider: experiment.provider,
    model: experiment.model,
    locality: experiment.locality,
    inputHash: experiment.inputHash,
    candidate: canonicalCandidate(experiment.candidate),
    pairs: experiment.pairs.map(canonicalPair),
    metrics: {
      pairsTotal: experiment.metrics.pairsTotal,
      pairsDone: experiment.metrics.pairsDone,
      correctOriginal: experiment.metrics.correctOriginal,
      correctCompacted: experiment.metrics.correctCompacted,
      abstentionsOriginal: experiment.metrics.abstentionsOriginal,
      abstentionsCompacted: experiment.metrics.abstentionsCompacted,
      failedPairs: experiment.metrics.failedPairs,
      tokensOriginal: experiment.metrics.tokensOriginal ?? null,
      tokensCompacted: experiment.metrics.tokensCompacted ?? null,
      tokensReportedOriginalPairs: experiment.metrics.tokensReportedOriginalPairs,
      tokensReportedCompactedPairs: experiment.metrics.tokensReportedCompactedPairs,
      compactionMicrosTotal: experiment.metrics.compactionMicrosTotal,
      totalLatencyMs: experiment.metrics.totalLatencyMs,
    },
    contentHash: '',
  });
}

export async function sha256Hex(text: string): Promise<string> {
  const bytes = await globalThis.crypto.subtle.digest(
    'SHA-256',
    new TextEncoder().encode(text),
  );
  return [...new Uint8Array(bytes)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}

export type CompactionApprovalInput = {
  cases: Array<{
    caseId: string;
    candidateHash: string;
    questions: CompactionQuestion[];
  }>;
  provider: string;
  model: string;
  /** Effective endpoint of the resolved target (configured baseUrl). */
  endpoint: string;
  locality: string;
  cost: string;
  cloud: boolean;
  maxTotalTokens: number;
};

/**
 * The approval hash (repair design §2 D2): SHA-256 hex of the canonical
 * approval JSON — the reviewed cases (case id, candidate hash,
 * questions), the RESOLVED provider/model (a blank default is resolved
 * before approval, never at start), the resolved target's effective
 * endpoint, locality, and cost class, the cloud flag, and the approved
 * budget. Key order must match the server's serde struct field order
 * exactly (cases, provider, model, endpoint, locality, cost, cloud,
 * maxTotalTokens; per case caseId, candidateHash, questions; per
 * question id, text, expectedAnswer); the e2e mirror builds the same
 * string. The server recomputes it at start from the live
 * configuration and refuses any mismatch before traffic.
 */
export async function computeCompactionApprovalHash(input: CompactionApprovalInput): Promise<string> {
  const canonical = JSON.stringify({
    cases: input.cases.map(entry => ({
      caseId: entry.caseId,
      candidateHash: entry.candidateHash,
      questions: entry.questions.map(question => ({
        id: question.id,
        text: question.text,
        expectedAnswer: question.expectedAnswer ?? null,
      })),
    })),
    provider: input.provider,
    model: input.model,
    endpoint: input.endpoint,
    locality: input.locality,
    cost: input.cost,
    cloud: input.cloud,
    maxTotalTokens: input.maxTotalTokens,
  });
  return sha256Hex(canonical);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isStringRecord(value: unknown, keys: string[]): value is Record<string, unknown> {
  return isRecord(value) && keys.every(key => typeof value[key] === 'string');
}

/**
 * Strict parser for a saved compaction experiment body. Rejects anything
 * that is not exactly format anansi-somnia-compaction-experiment v1 with
 * the fields the results view needs; a rejected body is never shown as a
 * result.
 */
export function parseSomniaCompactionExperiment(
  body: string,
): SomniaCompactionExperiment | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;
  if (parsed.format !== COMPACTION_EXPERIMENT_FORMAT || parsed.version !== 1) return null;
  if (
    typeof parsed.runId !== 'string' ||
    typeof parsed.provider !== 'string' ||
    typeof parsed.model !== 'string' ||
    typeof parsed.locality !== 'string' ||
    typeof parsed.inputHash !== 'string' ||
    typeof parsed.contentHash !== 'string'
  ) {
    return null;
  }
  if (!isRecord(parsed.candidate) || typeof parsed.candidate.contentHash !== 'string') {
    return null;
  }
  if (!Array.isArray(parsed.pairs)) return null;
  if (!isRecord(parsed.metrics)) return null;
  const metricKeys = [
    'pairsTotal', 'pairsDone', 'correctOriginal', 'correctCompacted',
    'abstentionsOriginal', 'abstentionsCompacted', 'failedPairs',
    'tokensReportedOriginalPairs', 'tokensReportedCompactedPairs',
    'compactionMicrosTotal', 'totalLatencyMs',
  ];
  const metrics = parsed.metrics;
  if (!metricKeys.every(key => typeof metrics[key] === 'number')) return null;
  return parsed as unknown as SomniaCompactionExperiment;
}

function inputEnvelope(records: SomniaCompactionRecord[]): string {
  return JSON.stringify(
    { format: 'anansi-somnia-compaction-input', version: 1, records },
    null,
    2,
  );
}

type ApprovedSnapshot = {
  caseId: string;
  candidate: SomniaCompactionCandidate;
  questions: CompactionQuestion[];
  candidateHash: string;
  /** The RESOLVED approved target (repair design §2 D1–D2): exact
   * provider/model/endpoint/locality/cost, as returned by the
   * resolve op at approve time. Never a blank default. */
  provider: string;
  model: string;
  endpoint: string;
  locality: string;
  cost: string;
  cloud: boolean;
  maxTotalTokens: number;
  approvalHash: string;
};

export function SomniaCompaction({ client, workspaceId, projectId, onSaved, busy }: {
  client: Client;
  workspaceId: string;
  projectId: string;
  onSaved: () => Promise<void>;
  busy: boolean;
}) {
  const [inputText, setInputText] = useState('');
  const [inputFormat, setInputFormat] = useState<'json' | 'text'>('json');
  const [questionsText, setQuestionsText] = useState('');
  const [benchmarkCase, setBenchmarkCase] = useState('');
  const [candidate, setCandidate] = useState<SomniaCompactionPrepareResponse | null>(null);
  const [approved, setApproved] = useState<ApprovedSnapshot | null>(null);
  const [approving, setApproving] = useState(false);
  const [provider, setProvider] = useState('ollama');
  const [model, setModel] = useState('');
  const [cloud, setCloud] = useState(false);
  const [maxTotalTokens, setMaxTotalTokens] = useState(MAX_TOTAL_TOKENS_DEFAULT);
  const [runId, setRunId] = useState<string | null>(null);
  const [poll, setPoll] = useState<SomniaCompactionPollResponse | null>(null);
  const [polling, setPolling] = useState(false);
  const [preparing, setPreparing] = useState(false);
  const [starting, setStarting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [savedId, setSavedId] = useState('');
  const [saved, setSaved] = useState<ArtifactRecord[]>([]);
  const [viewing, setViewing] = useState<SomniaCompactionExperiment | null>(null);
  const [error, setError] = useState('');
  const mounted = useRef(false);
  const runIdRef = useRef<string | null>(null);

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);

  useEffect(() => {
    let active = true;
    void client
      .listProjectArtifacts(workspaceId, projectId)
      .then(list => {
        if (active) {
          setSaved(list.artifacts.filter(artifact => artifact.type === 'somnia-compaction-experiment'));
        }
      })
      .catch(() => undefined);
    return () => { active = false; };
  }, [client, workspaceId, projectId]);

  // Poll while a run is in flight. The interval reads the run id from a
  // ref so the closure never goes stale.
  useEffect(() => {
    if (runId === null || !polling) return;
    const timer = setInterval(() => {
      const id = runIdRef.current;
      if (id === null) return;
      void client
        .pollSomniaCompaction(workspaceId, projectId, id)
        .then(result => {
          if (!mounted.current) return;
          setPoll(result);
          if (result.status !== 'running' && result.status !== 'cancelling') {
            setPolling(false);
          }
        })
        .catch(err => {
          if (!mounted.current) return;
          setPolling(false);
          setError(err instanceof Error ? err.message : 'Could not poll the compaction run.');
        });
    }, 1000);
    return () => clearInterval(timer);
  }, [client, workspaceId, projectId, runId, polling]);

  function editInput(patch: { text?: string; format?: 'json' | 'text'; questions?: string }) {
    if (patch.text !== undefined) setInputText(patch.text);
    if (patch.format !== undefined) setInputFormat(patch.format);
    if (patch.questions !== undefined) setQuestionsText(patch.questions);
    setCandidate(null);
    setApproved(null);
  }

  function editCompare(patch: { provider?: string; model?: string; cloud?: boolean; maxTotalTokens?: number }) {
    if (patch.provider !== undefined) setProvider(patch.provider);
    if (patch.model !== undefined) setModel(patch.model);
    if (patch.cloud !== undefined) setCloud(patch.cloud);
    if (patch.maxTotalTokens !== undefined) setMaxTotalTokens(patch.maxTotalTokens);
    setApproved(null);
  }

  function loadBenchmarkCase(caseId: string) {
    setBenchmarkCase(caseId);
    if (!caseId) return;
    const found = SOMNIA_COMPACTION_BENCHMARK_CASES.find(entry => entry.id === caseId);
    if (!found) return;
    editInput({
      text: inputEnvelope(found.records),
      format: 'json',
      questions: JSON.stringify(found.questions, null, 2),
    });
  }

  function parseQuestions(): CompactionQuestion[] | null {
    if (!questionsText.trim()) return [];
    try {
      const parsed: unknown = JSON.parse(questionsText);
      if (!Array.isArray(parsed)) return null;
      return parsed.map(entry => {
        if (!isStringRecord(entry, ['id', 'text'])) throw new Error('Bad question');
        return {
          id: entry.id as string,
          text: entry.text as string,
          expectedAnswer: typeof entry.expectedAnswer === 'string' ? entry.expectedAnswer : null,
        };
      });
    } catch {
      return null;
    }
  }

  async function prepare() {
    if (preparing || busy) return;
    const questions = parseQuestions();
    if (questions === null) {
      setError('Questions must be a JSON array of {id, text, expectedAnswer}.');
      return;
    }
    setPreparing(true);
    setError('');
    try {
      const result = await client.prepareSomniaCompaction(workspaceId, {
        projectId,
        inputText,
        inputFormat,
      });
      if (!mounted.current) return;
      setCandidate(result);
      setApproved(null);
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : 'Could not prepare the candidate.');
    } finally {
      if (mounted.current) setPreparing(false);
    }
  }

  async function approve() {
    if (!candidate || approving) return;
    const questions = parseQuestions();
    if (questions === null || questions.length === 0) {
      setError('Add at least one question before approving the comparison.');
      return;
    }
    const caseId = benchmarkCase || 'imported';
    setApproving(true);
    setError('');
    try {
      // Resolve the default BEFORE approval (repair design §2 D1):
      // a blank Model ID becomes the exact first configured model,
      // and the review shows that target — never "first configured
      // model". The approval hash binds the resolved values, so a
      // configuration change after approval requires a fresh review.
      const resolved = await client.resolveSomniaCompactionTarget(workspaceId, {
        projectId,
        provider: provider.trim() === '' ? undefined : provider.trim(),
        model: model.trim() === '' ? undefined : model.trim(),
      });
      const approvalHash = await computeCompactionApprovalHash({
        cases: [{ caseId, candidateHash: candidate.candidate.contentHash, questions }],
        provider: resolved.provider,
        model: resolved.model,
        endpoint: resolved.endpoint,
        locality: resolved.locality,
        cost: resolved.cost,
        cloud,
        maxTotalTokens,
      });
      if (!mounted.current) return;
      setApproved({
        caseId,
        candidate: candidate.candidate,
        questions,
        candidateHash: candidate.candidate.contentHash,
        provider: resolved.provider,
        model: resolved.model,
        endpoint: resolved.endpoint,
        locality: resolved.locality,
        cost: resolved.cost,
        cloud,
        maxTotalTokens,
        approvalHash,
      });
      setError('');
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : 'Could not resolve the comparison target.');
    } finally {
      if (mounted.current) setApproving(false);
    }
  }

  async function start() {
    if (!approved || starting || busy) return;
    const retainedRecords = (section: SomniaCompactionCandidate['retained']) =>
      [...section.trusted, ...section.unverified, ...section.conflicts, ...section.deletions, ...section.uncertainties]
        .map(entry => entry.record);
    const records = [
      ...retainedRecords(approved.candidate.retained),
      ...approved.candidate.excluded.map(entry => entry.record),
    ];
    setStarting(true);
    setError('');
    setSavedId('');
    setPoll(null);
    try {
      const result = await client.startSomniaCompaction(workspaceId, {
        projectId,
        cases: [{
          caseId: approved.caseId,
          records,
          quarantined: approved.candidate.quarantined,
          candidateHash: approved.candidateHash,
          questions: approved.questions,
        }],
        // The approved values, not the live form: the approval hash
        // covers exactly these resolved values, and the server
        // refuses any mismatch — including a configuration change
        // to the approved target after approval.
        provider: approved.provider,
        model: approved.model,
        cloud: approved.cloud,
        maxTotalTokens: approved.maxTotalTokens,
        approvalHash: approved.approvalHash,
      });
      if (!mounted.current) return;
      runIdRef.current = result.runId;
      setRunId(result.runId);
      setPolling(true);
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : 'Could not start the comparison.');
    } finally {
      if (mounted.current) setStarting(false);
    }
  }

  async function cancel() {
    const id = runIdRef.current;
    if (id === null) return;
    try {
      await client.cancelSomniaCompaction(workspaceId, projectId, id);
      const result = await client.pollSomniaCompaction(workspaceId, projectId, id);
      if (!mounted.current) return;
      setPoll(result);
      // The run thread only reaches its terminal status when the
      // in-flight dispatch returns, so keep polling until then.
      if (result.status !== 'running' && result.status !== 'cancelling') {
        setPolling(false);
      }
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : 'Could not cancel the run.');
    }
  }

  async function save() {
    if (!approved || !poll || poll.status !== 'complete' || saving || busy) return;
    setSaving(true);
    setError('');
    try {
      const metrics = computeCompactionMetrics(
        poll.pairs,
        poll.compactionMicrosTotal,
        poll.totalLatencyMs ?? 0,
      );
      const experiment: SomniaCompactionExperiment = {
        format: COMPACTION_EXPERIMENT_FORMAT,
        version: 1,
        runId: poll.runId,
        provider: poll.provider,
        model: poll.model,
        locality: poll.locality,
        inputHash: approved.candidate.inputHash,
        candidate: approved.candidate,
        pairs: poll.pairs,
        metrics,
        contentHash: '',
      };
      experiment.contentHash = await sha256Hex(canonicalCompactionExperimentJson(experiment));
      const artifact = await client.saveSomniaCompaction(workspaceId, {
        projectId,
        experiment,
      });
      if (!mounted.current) return;
      setSavedId(artifact.artifact.artifactId);
      await onSaved();
      const list = await client.listProjectArtifacts(workspaceId, projectId);
      if (mounted.current) {
        setSaved(list.artifacts.filter(entry => entry.type === 'somnia-compaction-experiment'));
      }
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : 'Could not save the experiment.');
    } finally {
      if (mounted.current) setSaving(false);
    }
  }

  async function viewSaved(artifactId: string, version: number) {
    try {
      const details = await client.readProjectArtifact(workspaceId, artifactId, version);
      const experiment = parseSomniaCompactionExperiment(details.body);
      if (experiment === null) {
        setError('The saved experiment could not be parsed.');
        return;
      }
      setViewing(experiment);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not open the experiment.');
    }
  }

  const running = polling && poll?.status !== 'complete';
  const finished = poll !== null && poll.status !== 'running' && poll.status !== 'cancelling';

  return <section aria-label="SOMNIA automatic compaction">
    <h4>Automatic compaction</h4>
    <p>Import memory records, review the compacted candidate, approve the exact reviewed inputs, then run the original and compacted histories against the same configured model. Nothing is sent to a model before approval.</p>
    <form onSubmit={event => { event.preventDefault(); void prepare(); }}>
      <fieldset disabled={busy || preparing || polling}>
        <label>Frozen benchmark case
          <select value={benchmarkCase} onChange={e => loadBenchmarkCase(e.target.value)}>
            <option value="">Choose a case…</option>
            {SOMNIA_COMPACTION_BENCHMARK_CASES.map(entry => (
              <option key={entry.id} value={entry.id}>{entry.id} · {entry.category} · {entry.questions.length} question{entry.questions.length === 1 ? '' : 's'}</option>
            ))}
          </select>
        </label>
        <label>Input format
          <select value={inputFormat} onChange={e => editInput({ format: e.target.value as 'json' | 'text' })}>
            <option value="json">JSON (anansi-somnia-compaction-input v1)</option>
            <option value="text">Text lines</option>
          </select>
        </label>
        <label>Records to compact <textarea required rows={8} value={inputText} onChange={e => editInput({ text: e.target.value })} /></label>
        <label>Questions (JSON array of {'{id, text, expectedAnswer}'}, expectedAnswer null when the history should not state it) <textarea rows={4} value={questionsText} onChange={e => editInput({ questions: e.target.value })} /></label>
        <button type="submit">Prepare candidate</button>
      </fieldset>
    </form>
    {candidate && <section aria-label="Compacted candidate review">
      <h5>Candidate review</h5>
      <p>Input hash <code>{candidate.candidate.inputHash.slice(0, 16)}…</code> · Candidate hash <code>{candidate.candidate.contentHash.slice(0, 16)}…</code></p>
      <p>Retained {candidate.candidate.stats.retainedRecords} · Excluded {candidate.candidate.stats.excludedRecords} · Quarantined {candidate.candidate.stats.quarantinedRecords} of {candidate.candidate.stats.inputRecords} records.</p>
      {(['trusted', 'unverified', 'conflicts', 'deletions', 'uncertainties'] as const).map(section => (
        candidate.candidate.retained[section].length > 0 && <details key={section}>
          <summary>Retained · {section} ({candidate.candidate.retained[section].length})</summary>
          <ul>{candidate.candidate.retained[section].map(entry => (
            <li key={entry.record.id}><strong>{entry.record.id}</strong> [{entry.record.key}] {entry.record.text} <em>({entry.status}{entry.detail ? ` — ${entry.detail}` : ''})</em></li>
          ))}</ul>
        </details>
      ))}
      {candidate.candidate.excluded.length > 0 && <details>
        <summary>Excluded ({candidate.candidate.excluded.length})</summary>
        <ul>{candidate.candidate.excluded.map(entry => (
          <li key={entry.record.id}><strong>{entry.record.id}</strong> [{entry.record.key}] {entry.record.text} <em>({entry.reason}{entry.detail ? ` — ${entry.detail}` : ''})</em></li>
        ))}</ul>
      </details>}
      {candidate.candidate.quarantined.length > 0 && <details>
        <summary>Quarantined ({candidate.candidate.quarantined.length})</summary>
        <ul>{candidate.candidate.quarantined.map(entry => (
          <li key={entry.recordRef}><em>{entry.reason}</em></li>
        ))}</ul>
      </details>}
      <form onSubmit={event => { event.preventDefault(); void approve(); }}>
        <fieldset disabled={busy || polling}>
          <label>Provider ID <input required maxLength={128} value={provider} onChange={e => editCompare({ provider: e.target.value })} /></label>
          <label>Model ID (leave blank for the first configured model) <input maxLength={256} value={model} onChange={e => editCompare({ model: e.target.value })} /></label>
          <label><input type="checkbox" checked={cloud} onChange={e => editCompare({ cloud: e.target.checked })} /> Authorize cloud transmission for this comparison</label>
          <p>Cloud comparisons run free models only — a paid or unknown-cost model is refused before any call, and a comparison never uses a single-use permit.</p>
          <label>Maximum total tokens <input type="number" min={1000} max={200000} value={maxTotalTokens} onChange={e => editCompare({ maxTotalTokens: Number(e.target.value) })} /></label>
          <button type="submit" disabled={Boolean(approved) || approving}>Approve reviewed inputs</button>
        </fieldset>
      </form>
      {approved && <p role="status">Approved candidate {approved.candidateHash.slice(0, 16)}… with {approved.questions.length} question{approved.questions.length === 1 ? '' : 's'} · {approved.provider} / {approved.model} ({approved.locality} · {approved.cost} · {approved.endpoint}) · {approved.cloud ? 'cloud transmission authorized' : 'local only'} · {approved.maxTotalTokens} tokens max. Any edit requires a fresh review.</p>}
      {approved && <button type="button" disabled={busy || starting || polling} onClick={() => void start()}>Run approved comparison</button>}
    </section>}
    {poll && <section aria-label="Compaction run results">
      <h5>Comparison run {poll.runId}</h5>
      <p role="status">Status: {poll.status} · {poll.provider} / {poll.model} ({poll.locality} · {poll.cost}) · {poll.pairsDone} of {poll.pairsTotal} pairs · {poll.callsMade} calls · {poll.tokensUsed} tokens used</p>
      {poll.error && <p role="alert">{poll.error}</p>}
      {running && <button type="button" onClick={() => void cancel()}>Cancel compaction run</button>}
      {finished && poll.pairs.length > 0 && <>
        <table>
          <thead><tr><th>Question</th><th>Original</th><th>Compacted</th></tr></thead>
          <tbody>{poll.pairs.map(pair => (
            <tr key={pair.pairId}>
              <td>{pair.question}</td>
              <td>{pair.originalReply ?? pair.failure ?? '—'} <em>({pair.originalScore}{pair.originalPrivacy ? ` · ${pair.originalPrivacy}` : ''})</em></td>
              <td>{pair.compactedReply ?? pair.failure ?? '—'} <em>({pair.compactedScore}{pair.compactedPrivacy ? ` · ${pair.compactedPrivacy}` : ''})</em></td>
            </tr>
          ))}</tbody>
        </table>
        <p>Correct: original {poll.pairs.filter(pair => pair.originalScore === 'correct').length}/{poll.pairs.length}, compacted {poll.pairs.filter(pair => pair.compactedScore === 'correct').length}/{poll.pairs.length} · Failed pairs: {poll.pairs.filter(pair => pair.originalScore === 'failed' || pair.compactedScore === 'failed').length} · Compaction: {poll.compactionMicrosTotal} µs total</p>
      </>}
      {poll.status === 'complete' && <button type="button" disabled={busy || saving || Boolean(savedId)} onClick={() => void save()}>Save compaction experiment to project</button>}
      {savedId && <p role="status">Saved private compaction experiment to project artifacts.</p>}
    </section>}
    {error && <p role="alert">{error}</p>}
    {saved.length > 0 && <section aria-label="Saved compaction experiments">
      <h5>Saved experiments</h5>
      <ul>{saved.map(artifact => (
        <li key={`${artifact.artifactId}-v${artifact.version}`}>
          {artifact.title}{' '}
          <button onClick={() => void viewSaved(artifact.artifactId, artifact.version)}>View experiment</button>
        </li>
      ))}</ul>
    </section>}
    {viewing && <section aria-label="Saved compaction experiment">
      <h5>Saved experiment {viewing.runId}</h5>
      <p>{viewing.provider} / {viewing.model} ({viewing.locality}) · Correct: original {viewing.metrics.correctOriginal}/{viewing.metrics.pairsTotal}, compacted {viewing.metrics.correctCompacted}/{viewing.metrics.pairsTotal} · Failed pairs: {viewing.metrics.failedPairs} · Tokens: original {viewing.metrics.tokensOriginal ?? 'not reported'}, compacted {viewing.metrics.tokensCompacted ?? 'not reported'} · Compaction: {viewing.metrics.compactionMicrosTotal} µs</p>
      <table>
        <thead><tr><th>Question</th><th>Original</th><th>Compacted</th></tr></thead>
        <tbody>{viewing.pairs.map(pair => (
          <tr key={pair.pairId}>
            <td>{pair.question}</td>
            <td>{pair.originalReply ?? pair.failure ?? '—'} <em>({pair.originalScore}{pair.originalPrivacy ? ` · ${pair.originalPrivacy}` : ''})</em></td>
            <td>{pair.compactedReply ?? pair.failure ?? '—'} <em>({pair.compactedScore}{pair.compactedPrivacy ? ` · ${pair.compactedPrivacy}` : ''})</em></td>
          </tr>
        ))}</tbody>
      </table>
      <button type="button" onClick={() => setViewing(null)}>Close experiment</button>
    </section>}
  </section>;
}
