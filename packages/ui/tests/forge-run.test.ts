import assert from 'node:assert/strict';
import test from 'node:test';
import { ForgeRunController, type ForgeClient } from '../src/forgeRun';
import type { ForgePollResponse } from '@anansi/contracts';

/**
 * M7 Slice 1: the Product Forge run lifecycle — the same
 * staleness discipline as the idea/opportunity controllers,
 * parameterized by stage (spec vs build) so each stage only
 * ever drives its own client operations.
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

function pollResponse(runId: string, status: string): ForgePollResponse {
  return {
    contractVersion: 1,
    runId,
    status,
    partialChars: 0,
  } as ForgePollResponse;
}

class FakeClient implements ForgeClient {
  specStarts: Deferred<{ runId: string }>[] = [];
  buildStarts: Deferred<{ runId: string }>[] = [];
  polls: Deferred<ForgePollResponse>[] = [];
  specCancels: string[] = [];
  buildCancels: string[] = [];

  startForgeSpec() {
    const d = deferred<{ runId: string }>();
    this.specStarts.push(d);
    return d.promise as ReturnType<ForgeClient['startForgeSpec']>;
  }
  startForgeBuild() {
    const d = deferred<{ runId: string }>();
    this.buildStarts.push(d);
    return d.promise as ReturnType<ForgeClient['startForgeBuild']>;
  }
  pollForgeSpec() {
    const d = deferred<ForgePollResponse>();
    this.polls.push(d);
    return d.promise as ReturnType<ForgeClient['pollForgeSpec']>;
  }
  pollForgeBuild() {
    const d = deferred<ForgePollResponse>();
    this.polls.push(d);
    return d.promise as ReturnType<ForgeClient['pollForgeBuild']>;
  }
  cancelForgeSpec(_workspaceId: string, runId: string) {
    this.specCancels.push(runId);
    return Promise.resolve(pollResponse(runId, 'cancelled')) as ReturnType<
      ForgeClient['cancelForgeSpec']
    >;
  }
  cancelForgeBuild(_workspaceId: string, runId: string) {
    this.buildCancels.push(runId);
    return Promise.resolve(pollResponse(runId, 'cancelled')) as ReturnType<
      ForgeClient['cancelForgeBuild']
    >;
  }
}

const specInput = {
  projectId: 'p',
  sourceArtifactId: 'a',
  sourceVersion: 1,
  conceptIndex: 0,
  purpose: 'p',
  intendedUsers: 'u',
  mustHave: 'm',
};

test('a spec controller drives only spec operations through completion', async () => {
  const client = new FakeClient();
  const controller = new ForgeRunController(client, 'ws', 'spec');
  const starting = controller.start(specInput);
  client.specStarts[0].resolve({ runId: 'run-1' });
  const started = await starting;
  assert.equal(started?.runId, 'run-1');
  assert.equal(client.buildStarts.length, 0);
  const polling = controller.pollOnce();
  client.polls[0].resolve(pollResponse('run-1', 'complete'));
  const polled = await polling;
  assert.equal(polled?.status, 'complete');
  assert.equal(controller.currentRunId, '');
});

test('superseding a run cancels it and drops its late start', async () => {
  const client = new FakeClient();
  const controller = new ForgeRunController(client, 'ws', 'build');
  const first = controller.start({ projectId: 'p', specArtifactId: 's', specVersion: 1 });
  const second = controller.start({ projectId: 'p', specArtifactId: 's', specVersion: 2 });
  client.buildStarts[0].resolve({ runId: 'stale' });
  assert.equal(await first, null);
  assert.deepEqual(client.buildCancels, ['stale']);
  client.buildStarts[1].resolve({ runId: 'fresh' });
  assert.equal((await second)?.runId, 'fresh');
  assert.equal(controller.currentRunId, 'fresh');
});

test('reset cancels the active run and invalidates in-flight polls', async () => {
  const client = new FakeClient();
  const controller = new ForgeRunController(client, 'ws', 'spec');
  const starting = controller.start(specInput);
  client.specStarts[0].resolve({ runId: 'run-9' });
  await starting;
  const polling = controller.pollOnce();
  await controller.reset();
  assert.deepEqual(client.specCancels, ['run-9']);
  client.polls[0].resolve(pollResponse('run-9', 'complete'));
  assert.equal(await polling, null);
  assert.equal(controller.currentRunId, '');
});
