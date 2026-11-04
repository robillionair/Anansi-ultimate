import { useEffect, useState } from 'react';
import type {
  Client,
  McpProbeResult,
  McpPrompt,
  McpPromptGetResult,
  McpPromptList,
  McpResourceList,
  McpResourceReadResult,
  McpServer,
  McpTool,
  McpToolCallResult,
} from '@anansi/contracts';
import {
  annotationLabels,
  assemblePromptArguments,
  assembleToolArguments,
  parseArgsLines,
  parseEnvLines,
  parseToolSchema,
  recordToolCall,
  validateMcpDraftForm,
  validatePromptArguments,
  validateToolArguments,
  type ToolCallHistoryEntry,
} from './mcp.ts';

// One tool's call form, authorization dialog, and result. Nothing
// here dispatches a call on its own: the only dispatch path is the
// "Authorize and Call Tool" button inside the dialog, for the exact
// parameters shown there. Cancel dispatches nothing.
function ToolCaller({
  server,
  tool,
  result,
  calling,
  onCallTool,
}: {
  server: McpServer;
  tool: McpTool;
  result: McpToolCallResult | undefined;
  calling: boolean;
  onCallTool: (
    serverId: string,
    toolName: string,
    args: Record<string, unknown>,
  ) => Promise<McpToolCallResult>;
}) {
  const schema = parseToolSchema(tool.inputSchema);
  const labels = annotationLabels(tool);
  const destructive = tool.annotations?.destructiveHint === true;
  const keyPrefix = `mcp-call-${server.id}-${tool.name}`;
  const [open, setOpen] = useState(false);
  const [values, setValues] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState('');
  const [dialogArgs, setDialogArgs] = useState<Record<string, unknown> | null>(null);
  const [callError, setCallError] = useState('');

  function review() {
    const { args, errors: assembleErrors } = assembleToolArguments(schema.fields, values);
    const errors = [...assembleErrors, ...validateToolArguments(tool.inputSchema, args)];
    if (errors.length) {
      setFormError(errors.join(' '));
      return;
    }
    setFormError('');
    setDialogArgs(args);
  }

  async function authorize() {
    if (!dialogArgs) return;
    setCallError('');
    try {
      await onCallTool(server.id, tool.name, dialogArgs);
      setDialogArgs(null);
      setOpen(false);
    } catch (e) {
      setCallError(e instanceof Error ? e.message : 'Tool call unavailable. Retry.');
    }
  }

  return (
    <div>
      <div className="actions">
        <button onClick={() => setOpen((was) => !was)}>{open ? 'Hide call form' : 'Call tool'}</button>
      </div>
      {open ? (
        <div>
          {schema.valid ? (
            schema.fields.map((field) => (
              <div key={field.name}>
                <label htmlFor={`${keyPrefix}-${field.name}`}>
                  {field.name} ({field.type}
                  {field.required ? ', required' : ''})
                </label>
                {field.type === 'boolean' ? (
                  <input
                    id={`${keyPrefix}-${field.name}`}
                    type="checkbox"
                    checked={values[field.name] === 'true'}
                    onChange={(e) =>
                      setValues((prev) => ({ ...prev, [field.name]: e.target.checked ? 'true' : 'false' }))
                    }
                  />
                ) : field.type === 'enum' ? (
                  <select
                    id={`${keyPrefix}-${field.name}`}
                    value={values[field.name] ?? ''}
                    onChange={(e) => setValues((prev) => ({ ...prev, [field.name]: e.target.value }))}
                  >
                    <option value="">—</option>
                    {field.options.map((option) => (
                      <option key={String(option)} value={String(option)}>
                        {String(option)}
                      </option>
                    ))}
                  </select>
                ) : field.type === 'number' || field.type === 'integer' ? (
                  <input
                    id={`${keyPrefix}-${field.name}`}
                    type="number"
                    value={values[field.name] ?? ''}
                    onChange={(e) => setValues((prev) => ({ ...prev, [field.name]: e.target.value }))}
                  />
                ) : field.type === 'any' ? (
                  <textarea
                    id={`${keyPrefix}-${field.name}`}
                    value={values[field.name] ?? ''}
                    onChange={(e) => setValues((prev) => ({ ...prev, [field.name]: e.target.value }))}
                  />
                ) : (
                  <input
                    id={`${keyPrefix}-${field.name}`}
                    type="text"
                    value={values[field.name] ?? ''}
                    onChange={(e) => setValues((prev) => ({ ...prev, [field.name]: e.target.value }))}
                  />
                )}
              </div>
            ))
          ) : (
            <p>No usable input schema: this tool is called with no parameters.</p>
          )}
          {formError ? <p>{formError}</p> : null}
          <div className="actions">
            <button onClick={review}>Review and authorize</button>
          </div>
        </div>
      ) : null}
      {dialogArgs ? (
        <div role="dialog" aria-label={`Authorize ${tool.name}`}>
          <h4>Authorize tool call</h4>
          <p>Server: {server.name}</p>
          <p>Tool: {tool.name}</p>
          {labels.length ? (
            <p>
              {labels.map((label) => (
                <span key={label} className="badge">
                  {label}
                </span>
              ))}
            </p>
          ) : null}
          {destructive ? (
            <p>Warning: this tool is marked destructive. It may change data outside ANANSI.</p>
          ) : null}
          <p>Exact parameters:</p>
          <pre>{JSON.stringify(dialogArgs, null, 2)}</pre>
          <div className="actions">
            <button disabled={calling} onClick={() => void authorize()}>
              Authorize and Call Tool
            </button>
            <button className="secondary" onClick={() => setDialogArgs(null)}>
              Cancel
            </button>
          </div>
        </div>
      ) : null}
      {callError ? <p>{callError}</p> : null}
      {result ? (
        <div>
          <p>
            Tool call {result.isError ? 'Error' : 'Success'}: {tool.name}
          </p>
          {result.text ? <p>{result.text}</p> : null}
          {(result.content ?? []).some((part) => part.type !== 'text') ? (
            <p>
              Non-text content parts:{' '}
              {(result.content ?? [])
                .filter((part) => part.type !== 'text')
                .map((part) => part.type)
                .join(', ')}
            </p>
          ) : null}
          {result.structuredContent !== undefined && result.structuredContent !== null ? (
            <pre>{JSON.stringify(result.structuredContent, null, 2)}</pre>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

export function ResourceViewer({ read }: { read: McpResourceReadResult }) {
  return (
    <div>
      <p>Resource: {read.uri}</p>
      {read.contents.length === 0 ? (
        <p>No content returned.</p>
      ) : (
        <ul aria-label={`Content of ${read.uri}`}>
          {read.contents.map((content, index) => (
            <li key={`${content.uri}-${index}`}>
              {content.mimeType ? <p>Type: {content.mimeType}</p> : null}
              {content.text !== undefined && content.text !== null ? (
                <p>{content.text}</p>
              ) : (
                <p>Binary content is not shown.</p>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function PromptViewer({ result }: { result: McpPromptGetResult }) {
  return (
    <div>
      <p>Prompt: {result.name}</p>
      {result.description ? <p>{result.description}</p> : null}
      {result.messages.length === 0 ? (
        <p>No messages returned.</p>
      ) : (
        <ul aria-label={`Messages of ${result.name}`}>
          {result.messages.map((message, index) => (
            <li key={index}>
              <p>{message.role}</p>
              {(message.content ?? []).map((part, partIndex) => (
                <p key={partIndex}>
                  {part.text !== undefined && part.text !== null
                    ? part.text
                    : '(non-text content not shown.)'}
                </p>
              ))}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

// One prompt's argument form and Get button. A click is the explicit
// opt-in: nothing here fetches on mount or on probe, and the rendered
// result is for review only — it is never forwarded anywhere.
function PromptGetter({
  server,
  prompt,
  getting,
  result,
  onGetPrompt,
}: {
  server: McpServer;
  prompt: McpPrompt;
  getting: boolean;
  result: McpPromptGetResult | undefined;
  onGetPrompt: (
    serverId: string,
    name: string,
    args: Record<string, string>,
  ) => Promise<McpPromptGetResult>;
}) {
  const keyPrefix = `mcp-prompt-${server.id}-${prompt.name}`;
  const [values, setValues] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState('');
  const [getError, setGetError] = useState('');

  async function get() {
    const args = assemblePromptArguments(values);
    const errors = validatePromptArguments(prompt, args);
    if (errors.length) {
      setFormError(errors.join(' '));
      return;
    }
    setFormError('');
    setGetError('');
    try {
      await onGetPrompt(server.id, prompt.name, args);
    } catch (e) {
      setGetError(e instanceof Error ? e.message : 'Prompt unavailable. Retry.');
    }
  }

  return (
    <div>
      {(prompt.arguments ?? []).map((arg) => (
        <div key={arg.name}>
          <label htmlFor={`${keyPrefix}-${arg.name}`}>
            {arg.name}
            {arg.required ? ' (required)' : ''}
          </label>
          <input
            id={`${keyPrefix}-${arg.name}`}
            type="text"
            value={values[arg.name] ?? ''}
            onChange={(e) => setValues((prev) => ({ ...prev, [arg.name]: e.target.value }))}
          />
        </div>
      ))}
      {formError ? <p>{formError}</p> : null}
      <div className="actions">
        <button disabled={getting} onClick={() => void get()}>
          Get prompt
        </button>
      </div>
      {getError ? <p>{getError}</p> : null}
      {result ? <PromptViewer result={result} /> : null}
    </div>
  );
}

export function McpView({
  servers,
  probes,
  busyId,
  onProbe,
  onRemove,
  callResults = {},
  callHistory = [],
  callingKey = null,
  onCallTool = async () => {
    throw new Error('Tool calls are unavailable in this view.');
  },
  resourceLists = {},
  resourceReads = {},
  resourceErrors = {},
  promptLists = {},
  promptResults = {},
  promptErrors = {},
  busyAction = null,
  onListResources = () => {},
  onReadResource = () => {},
  onListPrompts = () => {},
  onGetPrompt = async () => {
    throw new Error('Prompt retrieval is unavailable in this view.');
  },
}: {
  servers: McpServer[];
  probes: Record<string, McpProbeResult>;
  busyId: string | null;
  onProbe: (id: string) => void;
  onRemove: (id: string) => void;
  callResults?: Record<string, McpToolCallResult>;
  callHistory?: ToolCallHistoryEntry[];
  callingKey?: string | null;
  onCallTool?: (
    serverId: string,
    toolName: string,
    args: Record<string, unknown>,
  ) => Promise<McpToolCallResult>;
  resourceLists?: Record<string, McpResourceList>;
  resourceReads?: Record<string, McpResourceReadResult>;
  resourceErrors?: Record<string, string>;
  promptLists?: Record<string, McpPromptList>;
  promptResults?: Record<string, McpPromptGetResult>;
  promptErrors?: Record<string, string>;
  busyAction?: string | null;
  onListResources?: (serverId: string) => void;
  onReadResource?: (serverId: string, uri: string) => void;
  onListPrompts?: (serverId: string) => void;
  onGetPrompt?: (
    serverId: string,
    name: string,
    args: Record<string, string>,
  ) => Promise<McpPromptGetResult>;
}) {
  return (
    <section className="card" aria-labelledby="mcp-title">
      <h2 id="mcp-title">MCP servers</h2>
      <p>
        A tool runs only after you approve that exact call: open a tool, fill its parameters,
        review them in the authorization dialog, and click Authorize and Call Tool. Nothing is
        invoked automatically, and remote servers are never called.
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
                              <ToolCaller
                                server={server}
                                tool={tool}
                                result={callResults[`${server.id}:${tool.name}`]}
                                calling={callingKey === `${server.id}:${tool.name}`}
                                onCallTool={onCallTool}
                              />
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
                    <p>
                      Resource and prompt content comes from the external server. It is shown
                      here for your review only and is never sent anywhere automatically. Nothing
                      is fetched until you click List, Read, or Get below.
                    </p>
                    <h3>Resources</h3>
                    {resourceErrors[server.id] ? <p>{resourceErrors[server.id]}</p> : null}
                    {resourceLists[server.id] ? (
                      <div>
                        {(resourceLists[server.id].resources ?? []).length === 0 ? (
                          <p>No resources listed.</p>
                        ) : (
                          <ul aria-label={`Resources from ${server.name}`}>
                            {(resourceLists[server.id].resources ?? []).map((resource) => (
                              <li key={resource.uri}>
                                <strong>{resource.name}</strong>
                                <p>{resource.uri}</p>
                                {resource.description ? <p>{resource.description}</p> : null}
                                <div className="actions">
                                  <button
                                    disabled={busyAction === `read:${server.id}:${resource.uri}`}
                                    onClick={() => onReadResource(server.id, resource.uri)}
                                  >
                                    Read
                                  </button>
                                </div>
                                {resourceReads[`${server.id}:${resource.uri}`] ? (
                                  <ResourceViewer
                                    read={resourceReads[`${server.id}:${resource.uri}`]}
                                  />
                                ) : null}
                              </li>
                            ))}
                          </ul>
                        )}
                        {resourceLists[server.id].truncated ? (
                          <p>Resource list truncated at 100 entries.</p>
                        ) : null}
                      </div>
                    ) : (
                      <div className="actions">
                        <button
                          disabled={busyAction === `resources:${server.id}`}
                          onClick={() => onListResources(server.id)}
                        >
                          List resources
                        </button>
                      </div>
                    )}
                    <h3>Prompt templates</h3>
                    {promptErrors[server.id] ? <p>{promptErrors[server.id]}</p> : null}
                    {promptLists[server.id] ? (
                      <div>
                        {(promptLists[server.id].prompts ?? []).length === 0 ? (
                          <p>No prompts listed.</p>
                        ) : (
                          <ul aria-label={`Prompt templates from ${server.name}`}>
                            {(promptLists[server.id].prompts ?? []).map((prompt) => (
                              <li key={prompt.name}>
                                <strong>{prompt.name}</strong>
                                {prompt.description ? <p>{prompt.description}</p> : null}
                                <PromptGetter
                                  server={server}
                                  prompt={prompt}
                                  getting={busyAction === `get:${server.id}:${prompt.name}`}
                                  result={promptResults[`${server.id}:${prompt.name}`]}
                                  onGetPrompt={onGetPrompt}
                                />
                              </li>
                            ))}
                          </ul>
                        )}
                        {promptLists[server.id].truncated ? (
                          <p>Prompt list truncated at 100 entries.</p>
                        ) : null}
                      </div>
                    ) : (
                      <div className="actions">
                        <button
                          disabled={busyAction === `prompts:${server.id}`}
                          onClick={() => onListPrompts(server.id)}
                        >
                          List prompts
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {callHistory.length ? (
        <div>
          <h3>Recent tool calls</h3>
          <p>
            Session history only: timestamp, server, tool, and status. Parameters, results, and
            environment values are never logged here.
          </p>
          <ul aria-label="Recent tool calls">
            {callHistory.map((entry, index) => (
              <li key={`${entry.calledAt}-${entry.toolName}-${index}`}>
                {new Date(entry.calledAt).toLocaleString()} — {entry.serverName} — {entry.toolName} —{' '}
                {entry.status}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
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
  const [callResults, setCallResults] = useState<Record<string, McpToolCallResult>>({});
  const [callHistory, setCallHistory] = useState<ToolCallHistoryEntry[]>([]);
  const [callingKey, setCallingKey] = useState<string | null>(null);
  const [resourceLists, setResourceLists] = useState<Record<string, McpResourceList>>({});
  const [resourceReads, setResourceReads] = useState<Record<string, McpResourceReadResult>>({});
  const [resourceErrors, setResourceErrors] = useState<Record<string, string>>({});
  const [promptLists, setPromptLists] = useState<Record<string, McpPromptList>>({});
  const [promptResults, setPromptResults] = useState<Record<string, McpPromptGetResult>>({});
  const [promptErrors, setPromptErrors] = useState<Record<string, string>>({});
  const [busyAction, setBusyAction] = useState<string | null>(null);

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

  async function callTool(
    serverId: string,
    toolName: string,
    args: Record<string, unknown>,
  ): Promise<McpToolCallResult> {
    const key = `${serverId}:${toolName}`;
    setCallingKey(key);
    try {
      const result = await client.callMcpTool(workspaceId, serverId, toolName, args);
      setCallResults((previous) => ({ ...previous, [key]: result }));
      const server = servers.find((s) => s.id === serverId);
      setCallHistory((previous) =>
        recordToolCall(previous, {
          serverName: server?.name ?? serverId,
          toolName,
          status: result.isError ? 'Error' : 'Success',
          calledAt: Date.now(),
        }),
      );
      return result;
    } finally {
      setCallingKey(null);
    }
  }

  async function listResources(serverId: string) {
    const key = `resources:${serverId}`;
    setBusyAction(key);
    setResourceErrors((previous) => {
      const next = { ...previous };
      delete next[serverId];
      return next;
    });
    try {
      const list = await client.listMcpResources(workspaceId, serverId);
      setResourceLists((previous) => ({ ...previous, [serverId]: list }));
    } catch (e) {
      setResourceErrors((previous) => ({
        ...previous,
        [serverId]: e instanceof Error ? e.message : 'Resources unavailable. Retry.',
      }));
    } finally {
      setBusyAction(null);
    }
  }

  async function readResource(serverId: string, uri: string) {
    const action = `read:${serverId}:${uri}`;
    setBusyAction(action);
    setResourceErrors((previous) => {
      const next = { ...previous };
      delete next[serverId];
      return next;
    });
    try {
      const read = await client.readMcpResource(workspaceId, serverId, uri);
      setResourceReads((previous) => ({ ...previous, [`${serverId}:${uri}`]: read }));
    } catch (e) {
      setResourceErrors((previous) => ({
        ...previous,
        [serverId]: e instanceof Error ? e.message : 'Resource unavailable. Retry.',
      }));
    } finally {
      setBusyAction(null);
    }
  }

  async function listPrompts(serverId: string) {
    const key = `prompts:${serverId}`;
    setBusyAction(key);
    setPromptErrors((previous) => {
      const next = { ...previous };
      delete next[serverId];
      return next;
    });
    try {
      const list = await client.listMcpPrompts(workspaceId, serverId);
      setPromptLists((previous) => ({ ...previous, [serverId]: list }));
    } catch (e) {
      setPromptErrors((previous) => ({
        ...previous,
        [serverId]: e instanceof Error ? e.message : 'Prompts unavailable. Retry.',
      }));
    } finally {
      setBusyAction(null);
    }
  }

  async function getPrompt(
    serverId: string,
    name: string,
    args: Record<string, string>,
  ): Promise<McpPromptGetResult> {
    setBusyAction(`get:${serverId}:${name}`);
    try {
      const result = await client.getMcpPrompt(workspaceId, serverId, name, args);
      setPromptResults((previous) => ({ ...previous, [`${serverId}:${name}`]: result }));
      return result;
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <div>
      <section className="card" aria-labelledby="mcp-register-title">
        <h2 id="mcp-register-title">Register an MCP server</h2>
        <p>
          A server is local-first: a stdio server runs only when you connect it, with only the
          environment variables you list here. Remote (SSE) servers can be registered but cannot
          be probed, called, or read — external network access is disabled by default.
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
      <McpView
        servers={servers}
        probes={probes}
        busyId={busyId}
        onProbe={probe}
        onRemove={remove}
        callResults={callResults}
        callHistory={callHistory}
        callingKey={callingKey}
        onCallTool={callTool}
        resourceLists={resourceLists}
        resourceReads={resourceReads}
        resourceErrors={resourceErrors}
        promptLists={promptLists}
        promptResults={promptResults}
        promptErrors={promptErrors}
        busyAction={busyAction}
        onListResources={listResources}
        onReadResource={readResource}
        onListPrompts={listPrompts}
        onGetPrompt={getPrompt}
      />
    </div>
  );
}
