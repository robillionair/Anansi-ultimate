import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { McpView } from '../src/Mcp.tsx';
import {
  annotationLabels,
  assembleToolArguments,
  parseArgsLines,
  parseEnvLines,
  parseToolSchema,
  recordToolCall,
  validateMcpDraftForm,
  validateToolArguments,
} from '../src/mcp.ts';
import type { McpProbeResult, McpServer, McpTool } from '@anansi/contracts';

test('validateMcpDraftForm mirrors the backend rules', () => {
  const base = {
    name: 'Local',
    transport: 'stdio',
    command: 'node',
    args: [],
    env: {},
    url: '',
  };
  assert.deepEqual(validateMcpDraftForm(base), []);
  assert.ok(validateMcpDraftForm({ ...base, name: '   ' }).length > 0);
  assert.ok(validateMcpDraftForm({ ...base, command: '' }).length > 0);
  assert.ok(validateMcpDraftForm({ ...base, url: 'https://example.test/mcp' }).length > 0);
  assert.ok(validateMcpDraftForm({ ...base, transport: 'tcp' }).length > 0);
  const sseBad = {
    name: 'Remote',
    transport: 'sse',
    command: '',
    args: [],
    env: {},
    url: 'ftp://example.test/mcp',
  };
  assert.ok(validateMcpDraftForm(sseBad).length > 0);
  assert.deepEqual(validateMcpDraftForm({ ...sseBad, url: 'https://example.test/mcp' }), []);
  assert.ok(
    validateMcpDraftForm({ ...base, env: { 'BAD-KEY': 'v' } }).length > 0,
  );
});

test('parseEnvLines and parseArgsLines handle blanks and bad lines', () => {
  const { env, errors } = parseEnvLines('API_KEY=secret\n\nNOEQUALS\nEMPTY=\n');
  assert.equal(env.API_KEY, 'secret');
  assert.equal(env.EMPTY, '');
  assert.equal(errors.length, 1);
  assert.deepEqual(parseArgsLines('  --flag\n\nsecond\n'), ['--flag', 'second']);
});

test('parseToolSchema lists fields and tolerates bad schemas', () => {
  const parsed = parseToolSchema({
    type: 'object',
    properties: {
      q: { type: 'string', description: 'Query.' },
      limit: { type: 'integer' },
      mode: { enum: ['fast', 'slow'] },
    },
    required: ['q'],
  });
  assert.equal(parsed.valid, true);
  assert.deepEqual(
    parsed.fields.map((f) => `${f.name}:${f.type}:${f.required}`),
    ['q:string:true', 'limit:integer:false', 'mode:enum:false'],
  );
  assert.equal(parsed.fields[0].description, 'Query.');
  assert.equal(parseToolSchema(null).valid, false);
  assert.equal(parseToolSchema({ type: 'object' }).valid, false);
  assert.equal(parseToolSchema({ properties: 'nope' }).valid, false);
  assert.equal(parseToolSchema([]).valid, false);
});

test('annotationLabels maps only reported hints', () => {
  const tool = {
    name: 't',
    annotations: { readOnlyHint: true, destructiveHint: false, openWorldHint: false },
  } as McpTool;
  assert.deepEqual(annotationLabels(tool), ['Read-only', 'Closed world']);
  assert.deepEqual(annotationLabels({ name: 't' } as McpTool), []);
});

const server: McpServer = {
  contractVersion: 1,
  id: 'mcp-1',
  name: 'Fake MCP',
  transport: 'stdio',
  command: 'node',
  args: ['fake.mjs'],
  envKeys: ['API_KEY'],
};

const probe: McpProbeResult = {
  contractVersion: 1,
  serverId: 'mcp-1',
  healthy: true,
  protocolVersion: '2025-11-25',
  healthMethod: 'ping',
  serverName: 'Fake MCP',
  tools: [
    {
      name: 'search',
      description: 'Search memory.',
      inputSchema: {
        type: 'object',
        properties: { q: { type: 'string' } },
        required: ['q'],
      },
      annotations: { readOnlyHint: true },
    },
  ],
  prompts: [
    { name: 'summarize', description: 'Summarize.', arguments: [{ name: 'text', required: true }] },
  ],
  truncated: false,
};

test('validateToolArguments mirrors the backend subset', () => {
  const schema = {
    type: 'object',
    properties: {
      q: { type: 'string' },
      n: { type: 'integer' },
      mode: { type: 'string', enum: ['fast', 'deep'] },
    },
    required: ['q'],
  };
  assert.deepEqual(validateToolArguments(schema, { q: 'x' }), []);
  assert.ok(validateToolArguments(schema, {}).some((e) => e.includes('Missing required parameter: q')));
  assert.ok(validateToolArguments(schema, { q: 3 }).some((e) => e.includes("must be of type string")));
  assert.ok(validateToolArguments(schema, { q: 'x', n: 1.5 }).some((e) => e.includes('integer')));
  assert.deepEqual(validateToolArguments(schema, { q: 'x', n: 2 }), []);
  assert.ok(validateToolArguments(schema, { q: 'x', mode: 'slow' }).some((e) => e.includes('allowed values')));
  assert.deepEqual(validateToolArguments(schema, { q: 'x', mode: 'deep' }), []);
  assert.deepEqual(validateToolArguments(null, { anything: true }), []);
});

test('assembleToolArguments omits empties and parses typed fields', () => {
  const fields = parseToolSchema({
    type: 'object',
    properties: {
      q: { type: 'string' },
      n: { type: 'integer' },
      flag: { type: 'boolean' },
      mode: { enum: ['fast', 'deep'] },
      extra: {},
    },
    required: ['q'],
  }).fields;
  const { args, errors } = assembleToolArguments(fields, {
    q: 'cats',
    n: '3',
    flag: 'true',
    mode: 'deep',
    extra: '{"a":1}',
  });
  assert.deepEqual(errors, []);
  assert.deepEqual(args, { q: 'cats', n: 3, flag: true, mode: 'deep', extra: { a: 1 } });
  const omitted = assembleToolArguments(fields, { q: '', n: '', flag: 'false', mode: '', extra: '' });
  assert.deepEqual(omitted.args, { flag: false });
  const bad = assembleToolArguments(fields, { q: 'x', n: 'nope', extra: '{bad' });
  assert.equal(bad.errors.length, 2);
});

test('recordToolCall is newest-first, capped, and carries no parameters', () => {
  const first = { serverName: 'Fake MCP', toolName: 'search', status: 'Success' as const, calledAt: 1 };
  const second = { serverName: 'Fake MCP', toolName: 'write', status: 'Error' as const, calledAt: 2 };
  const history = recordToolCall(recordToolCall([], first), second);
  assert.deepEqual(
    history.map((h) => h.toolName),
    ['write', 'search'],
  );
  assert.ok(!('args' in history[0]) && !('text' in history[0]));
  let capped = history;
  for (let i = 0; i < 60; i++) {
    capped = recordToolCall(capped, { ...first, calledAt: 10 + i });
  }
  assert.equal(capped.length, 50);
});

test('McpView renders call buttons, results, and history', () => {
  const html = renderToStaticMarkup(
    React.createElement(McpView, {
      servers: [server],
      probes: { 'mcp-1': probe },
      busyId: null,
      onProbe: () => {},
      onRemove: () => {},
      callResults: {
        'mcp-1:search': {
          contractVersion: 1,
          serverId: 'mcp-1',
          toolName: 'search',
          isError: false,
          text: 'Search results for "cats".',
          content: [{ type: 'text', text: 'Search results for "cats".' }],
        },
      },
      callHistory: [
        { serverName: 'Fake MCP', toolName: 'search', status: 'Success', calledAt: 1700000000000 },
      ],
      onCallTool: async () => {
        throw new Error('unused in static markup');
      },
    }),
  );
  assert.ok(html.includes('Call tool'));
  assert.ok(html.includes('Tool call Success: search'));
  assert.ok(html.includes('Search results for &quot;cats&quot;.'));
  assert.ok(html.includes('Recent tool calls'));
  assert.ok(html.includes('Fake MCP — search — Success'));
  assert.ok(html.includes('never logged here'));
});

test('McpView renders empty state, server rows, and probe results', () => {
  const empty = renderToStaticMarkup(
    React.createElement(McpView, {
      servers: [],
      probes: {},
      busyId: null,
      onProbe: () => {},
      onRemove: () => {},
    }),
  );
  assert.ok(empty.includes('No MCP servers configured yet.'));
  assert.ok(empty.includes('A tool runs only after you approve that exact call'));

  const html = renderToStaticMarkup(
    React.createElement(McpView, {
      servers: [server],
      probes: { 'mcp-1': probe },
      busyId: null,
      onProbe: () => {},
      onRemove: () => {},
    }),
  );
  assert.ok(html.includes('Fake MCP'));
  assert.ok(html.includes('Environment keys: API_KEY'));
  assert.ok(html.includes('Healthy via ping'));
  assert.ok(html.includes('protocol 2025-11-25'));
  assert.ok(html.includes('search'));
  assert.ok(html.includes('q (string, required)'));
  assert.ok(html.includes('Read-only'));
  assert.ok(html.includes('summarize'));
  assert.ok(html.includes('text (required)'));

  const refused: McpProbeResult = {
    ...probe,
    healthy: false,
    tools: [],
    prompts: [],
    error: 'Remote MCP servers cannot be probed in this slice.',
  };
  const refusedHtml = renderToStaticMarkup(
    React.createElement(McpView, {
      servers: [{ ...server, id: 'mcp-2', name: 'Remote', transport: 'sse', url: 'https://example.test/mcp' }],
      probes: { 'mcp-2': refused },
      busyId: null,
      onProbe: () => {},
      onRemove: () => {},
    }),
  );
  assert.ok(refusedHtml.includes('Not healthy: Remote MCP servers cannot be probed in this slice.'));
  assert.ok(refusedHtml.includes('No tools discovered.'));
});
