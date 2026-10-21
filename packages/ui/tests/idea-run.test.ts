import assert from 'node:assert/strict';
import test from 'node:test';
import { IdeaRunController, type IdeasClient } from '../src/ideaRun';
import type { IdeasPollResponse } from '@anansi/contracts';

/**
 * M6 Slice 2 reliability repair, Finding 2: the idea-run
 * lifecycle — project switching cancels the old run, delayed
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

function pollResponse(runId: string, status: string): IdeasPollResponse {
  return {
    contractVersion: 1,
    runId,
    status,
    partialChars: 0,
  } as IdeasPollResponse;
}

class FakeClient implements IdeasClient {
  starts: Deferred<{ runId: string }>[] = [];
  polls: Deferred<IdeasPollResponse>[] = [];
  cancels: string[] = [];
  concurrentPolls = 0;
  maxConcurrentPolls = 0;

  startProjectIdeas() {
    const d = deferred<{ runId: string }>();
    this.starts.push(d);
    return d.promise as ReturnType<IdeasClient['startProjectIdeas']>;
  }

  pollProjectIdeas(_workspaceId: string, runId: string) {
    this.concurrentPolls += 1;
    this.maxConcurrentPolls = Math.max(this.maxConcurrentPolls, this.concurrentPolls);
    const d = deferred<IdeasPollResponse>();
    this.polls.push(d);
    return d.promise.finally(() => {
      this.concurrentPolls -= 1;
    }) as ReturnType<IdeasClient['pollProjectIdeas']>;
  }

  cancelProjectIdeas(_workspaceId: string, runId: string) {
    this.cancels.push(runId);
    return Promise.resolve(pollResponse(runId, 'cancelled')) as ReturnType<
      IdeasClient['cancelProjectIdeas']
    >;
  }
}

const input = {
  projectId: 'p'.repeat(32),
  topic: 't',
  customer: 'c',
  budget: 'b',
  technology: 'x',
  objective: 'o',
} as Parameters<IdeaRunController['start']>[0];

test('project switch cancels the tracked run and drops its delayed poll response', async () => {
  const fake = new FakeClient();
  const controller = new IdeaRunController(fake, 'ws');
  const starting = controller.start(input);
  fake.starts[0].resolve({ runId: 'run-a' });
  const started = await starting;
  assert.equal(started?.runId, 'run-a');

  // A poll goes in flight, then the user switches projects.
  const polling = controller.pollOnce();
  await controller.reset();
  assert.deepEqual(fake.cancels, ['run-a'], 'switching cancels the old run');
  assert.equal(controller.currentRunId, '');

  // The delayed poll response must not be applied.
  fake.polls[0].resolve(pollResponse('run-a', 'running'));
  assert.equal(await polling, null, 'stale poll response is dropped');
});

test('a start superseded while in flight is cancelled and dropped', async () => {
  const fake = new FakeClient();
  const controller = new IdeaRunController(fake, 'ws');
  const starting = controller.start(input);
  // The project changes before the start resolves.
  await controller.reset();
  fake.starts[0].resolve({ runId: 'run-late' });
  assert.equal(await starting, null, 'the superseded start yields no state');
  assert.deepEqual(fake.cancels, ['run-late'], 'the just-created run is cancelled, not orphaned');
  assert.equal(controller.currentRunId, '');
});

test('polling never overlaps: a tick during an in-flight poll is skipped', async () => {
  const fake = new FakeClient();
  const controller = new IdeaRunController(fake, 'ws');
  const starting = controller.start(input);
  fake.starts[0].resolve({ runId: 'run-a' });
  await starting;

  const first = controller.pollOnce();
  const second = controller.pollOnce();
  assert.equal(await second, null, 'the overlapping tick is skipped, not queued');
  assert.equal(fake.polls.length, 1, 'only one poll request was issued');
  fake.polls[0].resolve(pollResponse('run-a', 'running'));
  assert.equal((await first)?.status, 'running');
  assert.equal(fake.maxConcurrentPolls, 1, 'polls never run concurrently');

  // After the terminal response the run is no longer tracked.
  const third = controller.pollOnce();
  fake.polls[1].resolve(pollResponse('run-a', 'complete'));
  assert.equal((await third)?.status, 'complete');
  assert.equal(controller.currentRunId, '');
  assert.equal(await controller.pollOnce(), null);
});

test('cancel waits out an in-flight poll and its late response cannot resurrect the run', async () => {
  const fake = new FakeClient();
  const controller = new IdeaRunController(fake, 'ws');
  const starting = controller.start(input);
  fake.starts[0].resolve({ runId: 'run-a' });
  await starting;

  const polling = controller.pollOnce();
  const cancelling = controller.cancel();
  // The in-flight poll resolves as "running" after cancel was
  // requested: it must be dropped (cancel sequence moved on).
  fake.polls[0].resolve(pollResponse('run-a', 'running'));
  assert.equal(await polling, null);
  const cancelled = await cancelling;
  assert.equal(cancelled?.status, 'cancelled');
  assert.deepEqual(fake.cancels, ['run-a']);
  assert.equal(controller.currentRunId, '');
});
