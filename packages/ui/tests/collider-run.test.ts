import assert from 'node:assert/strict';
import test from 'node:test';
import { ColliderRunController, type ColliderClient } from '../src/colliderRun';
import type { ColliderPollResponse } from '@anansi/contracts';

/**
 * M8 Slice 1: the collider run lifecycle mirrors the idea-run
 * discipline — project switching cancels the old run, delayed
 * responses never overwrite the new project's state, and
 * polling never overlaps.
 */

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function pollResponse(runId: string, status: string): ColliderPollResponse {
  return {
    contractVersion: 1,
    runId,
    status,
    partialChars: 0,
  } as ColliderPollResponse;
}

class FakeClient implements ColliderClient {
  starts: Deferred<{ runId: string }>[] = [];
  polls: Deferred<ColliderPollResponse>[] = [];
  cancels: string[] = [];
  concurrentPolls = 0;
  maxConcurrentPolls = 0;

  startProjectCollider() {
    const d = deferred<{ runId: string }>();
    this.starts.push(d);
    return d.promise as ReturnType<ColliderClient['startProjectCollider']>;
  }

  pollProjectCollider(_workspaceId: string, runId: string) {
    this.concurrentPolls += 1;
    this.maxConcurrentPolls = Math.max(this.maxConcurrentPolls, this.concurrentPolls);
    const d = deferred<ColliderPollResponse>();
    this.polls.push(d);
    void runId;
    return d.promise.finally(() => {
      this.concurrentPolls -= 1;
    }) as ReturnType<ColliderClient['pollProjectCollider']>;
  }

  cancelProjectCollider(_workspaceId: string, runId: string) {
    this.cancels.push(runId);
    return Promise.resolve(pollResponse(runId, 'cancelled')) as ReturnType<
      ColliderClient['cancelProjectCollider']
    >;
  }
}

const input = {
  projectId: 'p'.repeat(32),
  beamA: 'Trading.',
  beamB: 'Mycelium.',
  zeroCostLock: true,
} as Parameters<ColliderRunController['start']>[0];

test('project switch cancels the tracked run and drops its delayed poll response', async () => {
  const fake = new FakeClient();
  const controller = new ColliderRunController(fake, 'ws');
  const first = controller.start(input);
  fake.starts[0].resolve({ runId: 'run-old' });
  assert.equal((await first)?.runId, 'run-old');
  const poll = controller.pollOnce();
  // A second start supersedes the first run before its poll settles.
  const second = controller.start(input);
  fake.starts[1].resolve({ runId: 'run-new' });
  assert.equal((await second)?.runId, 'run-new');
  fake.polls[0].resolve(pollResponse('run-old', 'running'));
  assert.equal(await poll, null);
  assert.deepEqual(fake.cancels, ['run-old']);
});

test('polling never overlaps', async () => {
  const fake = new FakeClient();
  const controller = new ColliderRunController(fake, 'ws');
  const started = controller.start(input);
  fake.starts[0].resolve({ runId: 'run-1' });
  assert.equal((await started)?.runId, 'run-1');
  const first = controller.pollOnce();
  const skipped = controller.pollOnce();
  assert.equal(await skipped, null);
  fake.polls[0].resolve(pollResponse('run-1', 'running'));
  assert.equal((await first)?.status, 'running');
  assert.equal(fake.maxConcurrentPolls, 1);
});

test('cancel waits for an in-flight poll and invalidates its response', async () => {
  const fake = new FakeClient();
  const controller = new ColliderRunController(fake, 'ws');
  const started = controller.start(input);
  fake.starts[0].resolve({ runId: 'run-1' });
  assert.equal((await started)?.runId, 'run-1');
  const poll = controller.pollOnce();
  const cancelled = controller.cancel();
  fake.polls[0].resolve(pollResponse('run-1', 'running'));
  assert.equal(await poll, null);
  assert.equal((await cancelled)?.status, 'cancelled');
  assert.deepEqual(fake.cancels, ['run-1']);
});

test('a start superseded while in flight cancels its just-created run', async () => {
  const fake = new FakeClient();
  const controller = new ColliderRunController(fake, 'ws');
  const first = controller.start(input);
  const second = controller.start(input);
  fake.starts[0].resolve({ runId: 'run-late' });
  assert.equal(await first, null);
  fake.starts[1].resolve({ runId: 'run-now' });
  assert.equal((await second)?.runId, 'run-now');
  assert.deepEqual(fake.cancels, ['run-late']);
});
