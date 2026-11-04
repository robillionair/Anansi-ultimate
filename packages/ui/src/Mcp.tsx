import { useEffect, useState } from 'react';
import type { Client, McpProbeResult, McpServer } from '@anansi/contracts';
import { annotationLabels, parseArgsLines, parseEnvLines, parseToolSchema, validateMcpDraftForm } from './mcp.ts';

export function McpView({
  servers,
  probes,
  busyId,
  onProbe,
  onRemove,
}: {
  servers: McpServer[];
  probes: Record<string, McpProbeResult>;
  busyId: string | null;
  onProbe: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  return (
    <section className="card" aria-labelledby="mcp-title">
      <h2 id="mcp-title">MCP servers</h2>
      <p>
        Tools are listed, never run, in this slice. Connecting a server runs only its configured
        local command; remote servers are refused. Calling a tool will require explicit approval in
        a later slice.
      </p>
      {servers.length === 0 ? (
        <p>No MCP servers configured yet.</p>
      ) : (
        <ul aria-label="MCP servers">
          {servers.map((server) => {
            const probe = probes[server.id];
            return (
              <li key={server.id}>
                <strong>{server.name}</strong>
                <p>
                  <span className="badge">{server.transport}</span>
                </p>
                {server.transport === 'stdio' ? (
                  <p>
                    Command: {server.command}
                    {server.args && server.args.length ? ` ${server.args.join(' ')}` : ''}
                  </p>
                ) : (
                  <p>URL: {server.url}</p>
                )}
                <p>
                  {server.envKeys && server.envKeys.length
                    ? `Environment keys: ${server.envKeys.join(', ')}`
                    : 'No environment variables.'}
                </p>
                <div className="actions">
                  <button disabled={busyId === server.id} onClick={() => onProbe(server.id)}>
                    {probe ? 'Refresh' : 'Connect'}
                  </button>
                  <button className="secondary" onClick={() => onRemove(server.id)}>
                    Remove
                  </button>
                </div>
                {probe && (
                  <div>
                    {probe.healthy ? (
                      <p>
                        Healthy via {probe.healthMethod}
                        {probe.protocolVersion ? ` (protocol ${probe.protocolVersion})` : ''}
                        {probe.serverName ? ` — ${probe.serverName}` : ''}
                      </p>
                    ) : (
                      <p>Not healthy: {probe.error ?? 'Probe failed.'}</p>
                    )}
                    <h3>Discovered tools ({(probe.tools ?? []).length})</h3>
                    {(probe.tools ?? []).length === 0 ? (
                      <p>No tools discovered.</p>
                    ) : (
                      <ul aria-label={`Tools from ${server.name}`}>
                        {(probe.tools ?? []).map((tool) => {
                          const schema = parseToolSchema(tool.inputSchema);
                          const labels = annotationLabels(tool);
                          return (
                            <li key={tool.name}>
                              <strong>{tool.name}</strong>
                              {tool.description ? <p>{tool.description}</p> : null}
                              {labels.length ? (
                                <p>
                                  {labels.map((label) => (
                                    <span key={label} className="badge">
                                      {label}
                                    </span>
                                  ))}
                                </p>
                              ) : null}
                              {schema.valid ? (
                                <ul aria-label={`Input schema for ${tool.name}`}>
                                  {schema.fields.map((field) => (
                                    <li key={field.name}>
                                      {field.name} ({field.type}
                                      {field.required ? ', required' : ''})
                                      {field.description ? ` — ${field.description}` : ''}
                                    </li>
                                  ))}
                                </ul>
                              ) : (
                                <p>No usable input schema.</p>
                              )}
                            </li>
                          );
                        })}
                      </ul>
                    )}
                    <h3>Discovered prompts ({(probe.prompts ?? []).length})</h3>
                    {(probe.prompts ?? []).length === 0 ? (
                      <p>No prompts discovered.</p>
                    ) : (
                      <ul aria-label={`Prompts from ${server.name}`}>
                        {(probe.prompts ?? []).map((prompt) => (
                          <li key={prompt.name}>
                            <strong>{prompt.name}</strong>
                            {prompt.description ? <p>{prompt.description}</p> : null}
                            {(prompt.arguments ?? []).length ? (
                              <p>
                                Arguments:{' '}
                                {(prompt.arguments ?? [])
                                  .map((a) => `${a.name}${a.required ? ' (required)' : ''}`)
                                  .join(', ')}
                              </p>
                            ) : null}
                          </li>
                        ))}
                      </ul>
                    )}
                    {probe.truncated ? (
                      <p>Tool or prompt list truncated at 100 entries.</p>
                    ) : null}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

export function Mcp({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [servers, setServers] = useState<McpServer[]>([]);
  const [probes, setProbes] = useState<Record<string, McpProbeResult>>({});
  const [busyId, setBusyId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [name, setName] = useState('');
  const [transport, setTransport] = useState('stdio');
  const [command, setCommand] = useState('');
  const [argsText, setArgsText] = useState('');
  const [envText, setEnvText] = useState('');
  const [url, setUrl] = useState('');

  async function refresh() {
    try {
      const list = await client.listMcpServers(workspaceId);
      setServers(list.servers);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'MCP servers unavailable. Retry.');
    }
  }
  useEffect(() => {
    void refresh();
  }, [client, workspaceId]);

  async function register(event: React.FormEvent) {
    event.preventDefault();
    const { env, errors: envErrors } = parseEnvLines(envText);
    const draft = {
      name,
      transport,
      command,
      args: parseArgsLines(argsText),
      env,
      url,
    };
    const errors = [...envErrors, ...validateMcpDraftForm(draft)];
    if (errors.length) {
      setError(errors.join(' '));
      return;
    }
    setBusy(true);
    setError('');
    setNotice('');
    try {
      await client.registerMcpServer(workspaceId, {
        name: name.trim(),
        transport,
        command: transport === 'stdio' ? command.trim() : undefined,
        args: transport === 'stdio' ? draft.args : [],
        env: transport === 'stdio' ? env : {},
        url: transport === 'sse' ? url.trim() : undefined,
      });
      setName('');
      setCommand('');
      setArgsText('');
      setEnvText('');
      setUrl('');
      setNotice('MCP server registered.');
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Registration unavailable. Retry.');
    } finally {
      setBusy(false);
    }
  }

  async function probe(id: string) {
    setBusyId(id);
    setError('');
    try {
      const result = await client.probeMcpServer(workspaceId, id);
      setProbes((previous) => ({ ...previous, [id]: result }));
      if (!result.healthy) setNotice('Probe finished: server not healthy. See the error below.');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Probe unavailable. Retry.');
    } finally {
      setBusyId(null);
    }
  }

  async function remove(id: string) {
    setError('');
    try {
      await client.removeMcpServer(workspaceId, id);
      setProbes((previous) => {
        const next = { ...previous };
        delete next[id];
        return next;
      });
      setNotice('MCP server removed.');
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Removal unavailable. Retry.');
    }
  }

  return (
    <div>
      <section className="card" aria-labelledby="mcp-register-title">
        <h2 id="mcp-register-title">Register an MCP server</h2>
        <p>
          A server is local-first: a stdio server runs only when you connect it, with only the
          environment variables you list here. Remote (SSE) servers can be registered but cannot be
          probed in this slice — external network access is disabled by default.
        </p>
        <form onSubmit={register}>
          <label htmlFor="mcp-name">Server name</label>
          <input id="mcp-name" type="text" value={name} onChange={(e) => setName(e.target.value)} />
          <label htmlFor="mcp-transport">Transport</label>
          <select
            id="mcp-transport"
            aria-label="Transport"
            value={transport}
            onChange={(e) => setTransport(e.target.value)}
          >
            <option value="stdio">stdio (local command)</option>
            <option value="sse">SSE (remote URL)</option>
          </select>
          {transport === 'stdio' ? (
            <>
              <label htmlFor="mcp-command">Command</label>
              <input
                id="mcp-command"
                type="text"
                value={command}
                onChange={(e) => setCommand(e.target.value)}
              />
              <label htmlFor="mcp-args">Arguments (one per line)</label>
              <textarea
                id="mcp-args"
                value={argsText}
                onChange={(e) => setArgsText(e.target.value)}
              />
              <label htmlFor="mcp-env">Environment (KEY=value, one per line)</label>
              <textarea
                id="mcp-env"
                value={envText}
                onChange={(e) => setEnvText(e.target.value)}
              />
            </>
          ) : (
            <>
              <label htmlFor="mcp-url">Server URL</label>
              <input id="mcp-url" type="text" value={url} onChange={(e) => setUrl(e.target.value)} />
            </>
          )}
          <div className="actions">
            <button disabled={busy} type="submit">
              Register server
            </button>
          </div>
        </form>
        {error ? <p>{error}</p> : null}
        {notice ? <p>{notice}</p> : null}
      </section>
      <McpView servers={servers} probes={probes} busyId={busyId} onProbe={probe} onRemove={remove} />
    </div>
  );
}
