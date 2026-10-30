import type {
  ColliderUrlFetchResponse,
  ColliderUrlSource,
} from '@anansi/contracts';

/**
 * The outcome of one URL fetch. It never rejects: a
 * failed fetch is an outcome like a successful one, so
 * the caller can gate failures by the same generation
 * identity as successes. An obsolete outcome must change
 * neither the error nor the loading state of a newer
 * fetch. This mirrors DocumentLoadSession exactly: the
 * two sessions are separate (one per source kind) so a
 * newer selection of either kind retires the other
 * kind's pending load when the panel invalidates both.
 */
export type UrlFetchOutcome = {
  generation: number;
  result: ColliderUrlFetchResponse | null;
  error: Error | null;
};

/**
 * Per-beam URL fetch session. Every new selection, edit,
 * source switch, clear, rejection, or cancel invalidates
 * pending fetches: `load` resolves to its result only
 * when its generation is still current, otherwise to
 * null, and to its error for a failed fetch, so a late
 * response — successful or failed — can never overwrite
 * newer user intent. Callers own the loading indicator:
 * a superseded fetch resolves without effect, and the
 * invalidating action clears the indicator itself.
 * Callers must check the outcome's generation before
 * applying either the result or the error.
 */
export class UrlFetchSession {
  private generation = 0;

  get current(): number {
    return this.generation;
  }

  invalidate(): void {
    this.generation += 1;
  }

  async load(
    fetch: () => Promise<ColliderUrlFetchResponse>,
  ): Promise<UrlFetchOutcome> {
    const generation = ++this.generation;
    try {
      const result = await fetch();
      return {
        generation,
        result: generation === this.generation ? result : null,
        error: null,
      };
    } catch (e) {
      return {
        generation,
        result: null,
        error: e instanceof Error ? e : Error('Fetching the URL failed.'),
      };
    }
  }
}

/** Provenance record for a fetched beam, without its text. */
export function urlSourceFromFetch(
  response: ColliderUrlFetchResponse,
): ColliderUrlSource {
  return {
    submittedUrl: response.submittedUrl,
    finalUrl: response.finalUrl,
    contentKind: response.contentKind,
    extraction: response.extraction,
    fetchedBytes: response.fetchedBytes,
    fetchedHash: response.fetchedHash,
    textHash: response.textHash,
    textChars: response.textChars,
    bom: response.bom,
  };
}
