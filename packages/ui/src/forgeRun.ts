import type {
  Client,
  ForgeBuildStartInput,
  ForgePollResponse,
  ForgeReviseStartInput,
  ForgeSpecStartInput,
} from '@anansi/contracts';

/** The client surface the forge run lifecycle needs. */
export type ForgeClient = Pick<
  Client,
  | 'startForgeSpec'
  | 'pollForgeSpec'
  | 'cancelForgeSpec'
  | 'startForgeBuild'
  | 'pollForgeBuild'
  | 'cancelForgeBuild'
  | 'startForgeRevise'
  | 'pollForgeRevise'
  | 'cancelForgeRevise'
>;

export type ForgeStage = 'spec' | 'build' | 'revise';

/**
 * Owns one Product Forge run lifecycle (specification or
 * source generation) for the Projects screen. The staleness
 * discipline mirrors IdeaRunController: an epoch increments
 * whenever the tracked run is superseded (project switch,
 * close, unmount, new generation); async continuations
 * capture epoch + run id + cancel sequence and apply
 * results only if all still match; polling is serialized;
 * cancelling waits for an in-flight poll to settle and
 * invalidates its response.
 */
export class ForgeRunController {
  private epoch = 0;
  private cancelSeq = 0;
  private activeRunId = '';
  private inFlight: Promise<unknown> | null = null;

  constructor(
    private readonly client: ForgeClient,
    private readonly workspaceId: string,
    private readonly stage: ForgeStage,
  ) {}

  get currentRunId(): string {
    return this.activeRunId;
  }

  private startCall(input: ForgeSpecStartInput | ForgeBuildStartInput | ForgeReviseStartInput) {
    if (this.stage === 'spec') {
      return this.client.startForgeSpec(this.workspaceId, input as ForgeSpecStartInput);
    }
    if (this.stage === 'revise') {
      return this.client.startForgeRevise(this.workspaceId, input as ForgeReviseStartInput);
    }
    return this.client.startForgeBuild(this.workspaceId, input as ForgeBuildStartInput);
  }

  private pollCall(runId: string) {
    if (this.stage === 'spec') {
      return this.client.pollForgeSpec(this.workspaceId, runId);
    }
    if (this.stage === 'revise') {
      return this.client.pollForgeRevise(this.workspaceId, runId);
    }
    return this.client.pollForgeBuild(this.workspaceId, runId);
  }

  private cancelCall(runId: string) {
    if (this.stage === 'spec') {
      return this.client.cancelForgeSpec(this.workspaceId, runId);
    }
    if (this.stage === 'revise') {
      return this.client.cancelForgeRevise(this.workspaceId, runId);
    }
    return this.client.cancelForgeBuild(this.workspaceId, runId);
  }

  async start(input: ForgeSpecStartInput | ForgeBuildStartInput | ForgeReviseStartInput) {
    const epoch = ++this.epoch;
    const previous = this.activeRunId;
    this.activeRunId = '';
    if (previous) {
      void this.cancelCall(previous).catch(() => undefined);
    }
    const started = await this.startCall(input);
    if (epoch !== this.epoch) {
      void this.cancelCall(started.runId).catch(() => undefined);
      return null;
    }
    this.activeRunId = started.runId;
    return started;
  }

  async pollOnce(): Promise<ForgePollResponse | null> {
    if (!this.activeRunId || this.inFlight) return null;
    const runId = this.activeRunId;
    const epoch = this.epoch;
    const cancelSeq = this.cancelSeq;
    const request = this.pollCall(runId);
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

  async cancel(): Promise<ForgePollResponse | null> {
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
    const polled = await this.cancelCall(runId);
    if (epoch !== this.epoch || runId !== this.activeRunId) return null;
    if (polled.status !== 'running' && polled.status !== 'cancelling') {
      this.activeRunId = '';
    }
    return polled;
  }

  async reset(): Promise<void> {
    this.epoch += 1;
    this.cancelSeq += 1;
    const runId = this.activeRunId;
    this.activeRunId = '';
    if (runId) {
      try {
        await this.cancelCall(runId);
      } catch {
        // Best effort: the core's run TTL is the backstop.
      }
    }
  }
}
