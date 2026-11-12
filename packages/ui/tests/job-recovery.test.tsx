// Recovery notice tests. Static markup cannot run effects or clicks,
// so the click-through is covered by the mock-API e2e spec; here we
// cover the shared notice's conditional rendering: the exact message,
// the attempt count, and when the Retry recovery button appears.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import { JobRecoveryNotice } from '../src/JobRecovery';
import type { Job } from '@anansi/contracts';

type Diagnostic = NonNullable<Job['recoveryDiagnostic']>;

const diagnostic: Diagnostic = {
  attempts: 3,
  message:
    'Recovery stopped because storage was unavailable. Check the storage folder and permissions, then choose Retry recovery.',
  reason: 'storageUnavailable',
};

function render(
  props: Partial<{
    diagnostic: Diagnostic | null | undefined;
    state: Job['state'];
    busy: boolean;
  }> = {},
) {
  return renderToStaticMarkup(
    <JobRecoveryNotice
      diagnostic={'diagnostic' in props ? props.diagnostic : diagnostic}
      state={props.state ?? 'Queued'}
      busy={props.busy ?? false}
      onRecover={() => {}}
    />,
  );
}

test('diagnosed Queued job shows the message, attempts, and Retry recovery', () => {
  const html = render();
  assert.match(html, /role="alert"/);
  assert.ok(html.includes(diagnostic.message));
  assert.ok(html.includes('Attempts used: 3.'));
  assert.ok(html.includes('>Retry recovery</button>'));
  assert.doesNotMatch(html, /disabled/);
});

test('diagnosed Paused job shows the message without the button', () => {
  const html = render({ state: 'Paused' });
  assert.ok(html.includes(diagnostic.message));
  assert.ok(html.includes('Attempts used: 3.'));
  // The message itself names Retry recovery; the button must be absent.
  assert.doesNotMatch(html, /<button/);
});

test('corrupt-storage diagnostic shows its own message', () => {
  const html = render({
    diagnostic: {
      attempts: 1,
      message:
        'Recovery stopped because storage could not be read. Keep the original files and restore a verified backup, then choose Retry recovery.',
      reason: 'corruptStorage',
    },
  });
  assert.ok(html.includes('restore a verified backup'));
  assert.ok(html.includes('Attempts used: 1.'));
  assert.ok(html.includes('>Retry recovery</button>'));
});

test('no diagnostic renders nothing', () => {
  assert.equal(render({ diagnostic: null }), '');
  assert.equal(render({ diagnostic: undefined }), '');
});

test('busy disables the Retry recovery button', () => {
  const html = render({ busy: true });
  assert.ok(html.includes('>Retry recovery</button>'));
  assert.match(html, /<button disabled/);
});
