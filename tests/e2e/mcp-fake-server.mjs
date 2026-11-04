// Minimal NDJSON stdio MCP server for the Slice 1 e2e spec. It speaks
// the deployed 2025-11-25 lifecycle: initialize, notifications,
// tools/list, prompts/list, ping. It is test-only and never ships
// as a product server.
import readline from 'node:readline';

const rl = readline.createInterface({ input: process.stdin });

function reply(id, result) {
  process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result }) + '\n');
}

rl.on('line', (line) => {
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    return;
  }
  if (message.method === 'initialize') {
    reply(message.id, {
      protocolVersion: '2025-11-25',
      capabilities: {},
      serverInfo: { name: 'Fake MCP', version: '1.0' },
    });
  } else if (message.method === 'tools/list') {
    reply(message.id, {
      tools: [
        {
          name: 'search',
          description: 'Search memory.',
          inputSchema: {
            type: 'object',
            properties: { q: { type: 'string', description: 'Query.' } },
            required: ['q'],
          },
          annotations: { readOnlyHint: true },
        },
        {
          name: 'write',
          description: 'Write a note.',
          inputSchema: {
            type: 'object',
            properties: { text: { type: 'string' } },
          },
          annotations: { destructiveHint: true },
        },
      ],
    });
  } else if (message.method === 'tools/call') {
    const name = message.params?.name;
    const args = message.params?.arguments ?? {};
    if (name === 'search') {
      reply(message.id, {
        content: [{ type: 'text', text: `Search results for "${args.q}".` }],
      });
    } else if (name === 'write') {
      reply(message.id, {
        content: [{ type: 'text', text: `Wrote "${args.text}".` }],
      });
    } else {
      process.stdout.write(
        JSON.stringify({
          jsonrpc: '2.0',
          id: message.id,
          error: { code: -32602, message: 'Unknown tool' },
        }) + '\n',
      );
    }
  } else if (message.method === 'prompts/list') {
    reply(message.id, {
      prompts: [
        {
          name: 'summarize',
          description: 'Summarize.',
          arguments: [{ name: 'text', required: true }],
        },
      ],
    });
  } else if (message.method === 'ping') {
    reply(message.id, {});
  }
  // notifications/initialized and anything else: no reply.
});
