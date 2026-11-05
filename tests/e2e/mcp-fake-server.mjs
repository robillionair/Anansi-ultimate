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
  } else if (message.method === 'resources/list') {
    reply(message.id, {
      resources: [
        { uri: 'file:///guide.md', name: 'Guide', description: 'A guide.', mimeType: 'text/markdown' },
        { uri: 'file:///notes.txt', name: 'Notes', mimeType: 'text/plain' },
      ],
    });
  } else if (message.method === 'resources/read') {
    const uri = message.params?.uri;
    if (uri === 'file:///guide.md') {
      reply(message.id, {
        contents: [{ uri, mimeType: 'text/markdown', text: '# Guide\n\nReview me.' }],
      });
    } else if (uri === 'file:///notes.txt') {
      reply(message.id, {
        contents: [{ uri, mimeType: 'text/plain', text: 'Short notes.' }],
      });
    } else {
      process.stdout.write(
        JSON.stringify({
          jsonrpc: '2.0',
          id: message.id,
          error: { code: -32602, message: 'Unknown resource' },
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
        {
          name: 'greet',
          description: 'Greet.',
          arguments: [{ name: 'name', required: false }],
        },
      ],
    });
  } else if (message.method === 'prompts/get') {
    const name = message.params?.name;
    const args = message.params?.arguments ?? {};
    if (name === 'summarize' && typeof args.text === 'string' && args.text !== '') {
      reply(message.id, {
        description: 'Summarized.',
        messages: [{ role: 'user', content: [{ type: 'text', text: `Summary of "${args.text}".` }] }],
      });
    } else if (name === 'greet') {
      const who = typeof args.name === 'string' && args.name !== '' ? args.name : 'friend';
      reply(message.id, {
        messages: [{ role: 'assistant', content: [{ type: 'text', text: `Hello, ${who}!` }] }],
      });
    } else {
      process.stdout.write(
        JSON.stringify({
          jsonrpc: '2.0',
          id: message.id,
          error: { code: -32602, message: 'Unknown prompt or missing argument' },
        }) + '\n',
      );
    }
  } else if (message.method === 'ping') {
    reply(message.id, {});
  }
  // notifications/initialized and anything else: no reply.
});
