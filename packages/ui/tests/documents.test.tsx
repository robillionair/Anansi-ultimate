import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import { renderMarkdown } from '../src/markdown.ts';
import {
  ingestionJobLabel,
  ingestionReceived,
  isIngestionJob,
  prepareDocument,
} from '../src/documents.ts';
import type { Job } from '@anansi/contracts';

/**
 * M13 Document Ingestion Slice 1: the Markdown subset renderer is
 * inert (no anchors, no HTML execution), the document helpers label
 * ingestion jobs and read upload checkpoints, and the local decoder
 * refuses unsupported formats, oversize files and invalid UTF-8
 * before any network traffic.
 */

function job(overrides: Partial<Job> = {}): Job {
  return {
    id: 'job-1',
    workspaceId: 'ws-1',
    state: 'Paused',
    version: 1,
    checkpointVersion: 1,
    checkpoint: 'uploading:128',
    selection: {
      kind: 'ingestion',
      uploadId: 'job-1',
      fileName: 'notes.txt',
      format: 'txt',
      expectedBytes: 256,
      expectedHash: 'a'.repeat(64),
      privacy: 'private',
      projectId: null,
    },
    progress: 50,
    maxRecords: 10000,
    maxBytes: 268435456,
    artifactId: null,
    artifactBytes: null,
    artifactHash: null,
    createdAt: '2026-11-05T00:00:00Z',
    updatedAt: '2026-11-05T00:00:00Z',
    log: 'Upload in progress',
    ...overrides,
  };
}

test('markdown renderer produces headings, emphasis, code and lists as elements', () => {
  const html = renderToStaticMarkup(
    <div>{renderMarkdown('# Title\n\nSome **bold** and *italic* text with `code`.\n\n- one\n- two\n\n```\nconst x = 1;\n```\n')}</div>,
  );
  assert.ok(html.includes('<h3>Title</h3>'));
  assert.ok(html.includes('<strong>bold</strong>'));
  assert.ok(html.includes('<em>italic</em>'));
  assert.ok(html.includes('<code>code</code>'));
  assert.ok(html.includes('<ul><li>one</li><li>two</li></ul>'));
  assert.ok(html.includes('<pre><code>const x = 1;</code></pre>'));
});

test('markdown renderer never emits anchors or executes HTML', () => {
  const html = renderToStaticMarkup(
    <div>{renderMarkdown('[click](https://example.invalid)\n\n<script>alert(1)</script>\n\n<img src="x" onerror="alert(2)">\n')}</div>,
  );
  assert.doesNotMatch(html, /<a[\s>]/);
  assert.doesNotMatch(html, /<script/);
  assert.doesNotMatch(html, /<img/);
  assert.ok(html.includes('click (https://example.invalid)'));
  assert.ok(html.includes('&lt;script&gt;'));
});

test('ingestion job helpers read the checkpoint and label the file', () => {
  const uploading = job();
  assert.equal(isIngestionJob(uploading), true);
  assert.equal(ingestionReceived(uploading), 128);
  assert.equal(ingestionJobLabel(uploading), 'Document import: notes.txt');
  const staged = job({ checkpoint: 'staged' });
  assert.equal(ingestionReceived(staged), null);
  const exported = job({
    selection: { kind: 'workspace' },
    checkpoint: 'snapshot',
  });
  assert.equal(isIngestionJob(exported), false);
  assert.equal(ingestionJobLabel(exported), 'Workspace export');
});

test('prepareDocument refuses an unsupported format before any upload', async () => {
  const file = new File([new Uint8Array([37, 80, 68, 70])], 'report.pdf', {
    type: 'application/pdf',
  });
  await assert.rejects(() => prepareDocument(file), /Only \.txt and \.md/);
});

test('prepareDocument refuses invalid UTF-8 and oversize files', async () => {
  const bad = new File([new Uint8Array([0xff, 0xfe, 0x41])], 'bad.txt', { type: 'text/plain' });
  await assert.rejects(() => prepareDocument(bad), /not valid UTF-8/);
  const big = new File([new Uint8Array(65537).fill(97)], 'big.txt', { type: 'text/plain' });
  await assert.rejects(() => prepareDocument(big), /at most 65536 bytes/);
});

test('prepareDocument decodes a Markdown file with hashes and BOM recorded', async () => {
  const bytes = new Uint8Array([0xef, 0xbb, 0xbf, 35, 32, 72, 105]);
  const file = new File([bytes], 'note.md', { type: 'text/markdown' });
  const pending = await prepareDocument(file);
  assert.equal(pending.text, '# Hi');
  assert.equal(pending.kind, 'md');
  assert.equal(pending.bom, true);
  assert.equal(pending.rawBytes, 7);
  assert.match(pending.rawHash, /^[a-f0-9]{64}$/);
  assert.match(pending.textHash, /^[a-f0-9]{64}$/);
  assert.notEqual(pending.rawHash, pending.textHash);
});
