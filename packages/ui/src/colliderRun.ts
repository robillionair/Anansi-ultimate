import type {
  Client,
  ColliderPollResponse,
  ColliderStartInput,
} from '@anansi/contracts';

/** The client surface the run lifecycle needs. */
export type ColliderClient = Pick<
  Client,
  'startProjectCollider' | 'pollProjectCollider' | 'cancelProjectCollider'
>;

/**
 * Owns the collider run lifecycle for the Projects
 * screen: starting, serialized polling, cancellation, and
 * protection against stale responses. A direct mirror of
 * IdeaRunController, so the same staleness discipline
 * applies: an epoch increments whenever the tracked run
 * is superseded — a project switch, closing the project,
 * unmounting, or a new collision. Async continuations
 * capture the epoch (and the run id, and a cancel
 * sequence) when they start and apply their result only
 * if all still match, so a delayed response for a
 * previous project can never overwrite the newly
 * selected project's state.
 *
 * Polling is serialized: a tick that arrives while a poll
 * is in flight is skipped, never overlapped. Cancelling
 * waits for an in-flight poll to settle and invalidates
 * its response, so a late "running" can never resurrect
 * a run the user already cancelled.
 */
export class ColliderRunController {
  private epoch = 0;
  private cancelSeq = 0;
  private activeRunId = '';
  private inFlight: Promise<unknown> | null = null;

  constructor(
    private readonly client: ColliderClient,
    private readonly workspaceId: string,
  ) {}

  get currentRunId(): string {
    return this.activeRunId;
  }

  /**
   * Start a collision, superseding any active run (which
   * is cancelled first). Returns the start response, or
   * null when the start was superseded while in flight —
   * in that case the just-created run is cancelled and
   * the result dropped.
   */
  async start(input: ColliderStartInput) {
    const epoch = ++this.epoch;
    const previous = this.activeRunId;
    this.activeRunId = '';
    if (previous) {
      void this.client
        .cancelProjectCollider(this.workspaceId, previous)
        .catch(() => undefined);
    }
    const started = await this.client.startProjectCollider(this.workspaceId, input);
    if (epoch !== this.epoch) {
      void this.client
        .cancelProjectCollider(this.workspaceId, started.runId)
        .catch(() => undefined);
      return null;
    }
    this.activeRunId = started.runId;
    return started;
  }

  /**
   * One poll tick. Resolves to the response when it is
   * still current, or null when there is nothing to
   * poll, a poll is already in flight, or the response
   * arrived stale.
   */
  async pollOnce(): Promise<ColliderPollResponse | null> {
    if (!this.activeRunId || this.inFlight) return null;
    const runId = this.activeRunId;
    const epoch = this.epoch;
    const cancelSeq = this.cancelSeq;
    const request = this.client.pollProjectCollider(this.workspaceId, runId);
    this.inFlight = request;
    try {
      const polled = await request;
      if (
        epoch !== this.epoch ||
        cancelSeq !== this.cancelSeq ||
        runId !== this.activeRunId
      ) {
        return null;
      }
      if (polled.status !== 'running' && polled.status !== 'cancelling') {
        this.activeRunId = '';
      }
      return polled;
    } finally {
      if (this.inFlight === request) this.inFlight = null;
    }
  }

  /**
   * Cancel the tracked run. Waits for an in-flight poll
   * to settle first (its response is invalidated), then
   * issues the cancel. Returns the cancel response when
   * still current, else null.
   */
  async cancel(): Promise<ColliderPollResponse | null> {
    if (!this.activeRunId) return null;
    const runId = this.activeRunId;
    const epoch = this.epoch;
    this.cancelSeq += 1;
    if (this.inFlight) {
      try {
        await this.inFlight;
      } catch {
        // The poll's own caller handles its failure.
      }
    }
    if (epoch !== this.epoch || runId !== this.activeRunId) return null;
    const polled = await this.client.cancelProjectCollider(this.workspaceId, runId);
    if (epoch !== this.epoch || runId !== this.activeRunId) return null;
    if (polled.status !== 'running' && polled.status !== 'cancelling') {
      this.activeRunId = '';
    }
    return polled;
  }

  /**
   * Supersede whatever is tracked — project switch,
   * close, unmount. The active run, if any, is
   * cancelled; in-flight responses are invalidated by
   * the epoch bump.
   */
  async reset(): Promise<void> {
    this.epoch += 1;
    this.cancelSeq += 1;
    const runId = this.activeRunId;
    this.activeRunId = '';
    if (runId) {
      try {
        await this.client.cancelProjectCollider(this.workspaceId, runId);
      } catch {
        // Best effort: the core's run TTL is the backstop.
      }
    }
  }
}
