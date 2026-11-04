import type { McpTool } from '@anansi/contracts';

// Pure MCP helpers, mirroring the Rust validation in
// crates/core/src/mcp.rs. The backend remains authoritative; these
// helpers give the form immediate feedback and render tool schemas.

export interface McpDraftForm {
  name: string;
  transport: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  url: string;
}

export function validateMcpDraftForm(draft: McpDraftForm): string[] {
  const errors: string[] = [];
  const name = draft.name.trim();
  if (!name || name.length > 100 || [...name].some((c) => c.charCodeAt(0) < 32)) {
    errors.push('Server name must be 1 to 100 characters without control characters.');
  }
  if (draft.transport === 'stdio') {
    const command = draft.command.trim();
    if (!command || [...command].some((c) => c.charCodeAt(0) < 32)) {
      errors.push('A stdio server requires a command without control characters.');
    }
    if (draft.args.length > 64 || draft.args.some((a) => a.length > 1024)) {
      errors.push('Arguments are limited to 64 entries of 1024 characters.');
    }
    if (draft.url.trim()) errors.push('A stdio server takes no URL.');
  } else if (draft.transport === 'sse') {
    if (draft.command.trim() || draft.args.length || Object.keys(draft.env).length) {
      errors.push('An SSE server takes a URL only, not a command, arguments, or environment.');
    }
    try {
      const parsed = new URL(draft.url.trim());
      if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
        errors.push('An SSE server requires an http(s) URL.');
      }
    } catch {
      errors.push('An SSE server requires an http(s) URL.');
    }
  } else {
    errors.push("Transport must be 'stdio' or 'sse'.");
  }
  const keys = Object.keys(draft.env);
  if (
    keys.length > 50 ||
    keys.some((k) => !/^[A-Za-z0-9_]{1,128}$/.test(k)) ||
    Object.values(draft.env).some((v) => v.length > 4096)
  ) {
    errors.push('Environment is limited to 50 entries with alphanumeric/underscore keys.');
  }
  return errors;
}

export function parseEnvLines(text: string): { env: Record<string, string>; errors: string[] } {
  const env: Record<string, string> = {};
  const errors: string[] = [];
  for (const raw of text.split('\n')) {
    const line = raw.trim();
    if (!line) continue;
    const eq = line.indexOf('=');
    if (eq <= 0) {
      errors.push(`Environment line is not KEY=value: ${line.slice(0, 64)}`);
      continue;
    }
    env[line.slice(0, eq).trim()] = line.slice(eq + 1);
  }
  return { env, errors };
}

export function parseArgsLines(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

export interface SchemaField {
  name: string;
  type: string;
  description: string;
  required: boolean;
  options: unknown[];
}

export interface ParsedToolSchema {
  valid: boolean;
  fields: SchemaField[];
}

// A tool input schema is a JSON Schema object. Anything missing or
// malformed renders as "no usable input schema" rather than
// crashing the registry view: schemas are untrusted server text.
export function parseToolSchema(schema: unknown): ParsedToolSchema {
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    return { valid: false, fields: [] };
  }
  const record = schema as Record<string, unknown>;
  const properties = record.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) {
    return { valid: false, fields: [] };
  }
  const required = Array.isArray(record.required)
    ? record.required.filter((r): r is string => typeof r === 'string')
    : [];
  const fields: SchemaField[] = [];
  for (const [name, prop] of Object.entries(properties as Record<string, unknown>)) {
    if (!prop || typeof prop !== 'object' || Array.isArray(prop)) {
      fields.push({ name, type: 'any', description: '', required: required.includes(name), options: [] });
      continue;
    }
    const p = prop as Record<string, unknown>;
    const type =
      typeof p.type === 'string'
        ? p.type
        : Array.isArray(p.enum)
          ? 'enum'
          : 'any';
    fields.push({
      name,
      type,
      description: typeof p.description === 'string' ? p.description : '',
      required: required.includes(name),
      options: Array.isArray(p.enum) ? p.enum : [],
    });
  }
  return { valid: true, fields };
}

export function annotationLabels(tool: McpTool): string[] {
  const labels: string[] = [];
  const a = tool.annotations;
  if (!a) return labels;
  if (a.readOnlyHint) labels.push('Read-only');
  if (a.destructiveHint) labels.push('Destructive');
  if (a.idempotentHint) labels.push('Idempotent');
  if (a.openWorldHint === true) labels.push('Open world');
  if (a.openWorldHint === false) labels.push('Closed world');
  return labels;
}

// TS mirror of the Rust `validate_tool_arguments` subset: required
// presence, per-property types, enum membership. The backend
// remains authoritative; this gives the form immediate feedback.
export function validateToolArguments(schema: unknown, args: Record<string, unknown>): string[] {
  const errors: string[] = [];
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) return errors;
  const record = schema as Record<string, unknown>;
  const required = Array.isArray(record.required)
    ? record.required.filter((r): r is string => typeof r === 'string')
    : [];
  for (const name of required) {
    if (!(name in args)) errors.push(`Missing required parameter: ${name}`);
  }
  const properties = record.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return errors;
  for (const [name, value] of Object.entries(args)) {
    const prop = (properties as Record<string, unknown>)[name];
    if (!prop || typeof prop !== 'object' || Array.isArray(prop)) continue;
    const p = prop as Record<string, unknown>;
    if (Array.isArray(p.enum)) {
      const allowed = p.enum.some((v) => JSON.stringify(v) === JSON.stringify(value));
      if (!allowed) {
        errors.push(`Parameter '${name}' must be one of the allowed values.`);
        continue;
      }
    }
    if (typeof p.type !== 'string') continue;
    const ok =
      p.type === 'string'
        ? typeof value === 'string'
        : p.type === 'integer'
          ? typeof value === 'number' && Number.isInteger(value)
          : p.type === 'number'
            ? typeof value === 'number'
            : p.type === 'boolean'
              ? typeof value === 'boolean'
              : p.type === 'array'
                ? Array.isArray(value)
                : p.type === 'object'
                  ? value !== null && typeof value === 'object' && !Array.isArray(value)
                  : true;
    if (!ok) errors.push(`Parameter '${name}' must be of type ${p.type}.`);
  }
  return errors;
}

// Build the exact arguments object from raw form strings. Empty
// fields are omitted, so a required field left empty fails
// validation as missing rather than sending an empty value; an
// optional field left empty simply stays out of the call.
export function assembleToolArguments(
  fields: SchemaField[],
  values: Record<string, string>,
): { args: Record<string, unknown>; errors: string[] } {
  const args: Record<string, unknown> = {};
  const errors: string[] = [];
  for (const field of fields) {
    const raw = values[field.name] ?? '';
    if (field.type === 'boolean') {
      args[field.name] = raw === 'true';
      continue;
    }
    if (raw === '') continue;
    if (field.type === 'number' || field.type === 'integer') {
      const parsed = Number(raw);
      if (Number.isNaN(parsed)) {
        errors.push(`Parameter '${field.name}' must be of type ${field.type}.`);
        continue;
      }
      args[field.name] = parsed;
      continue;
    }
    if (field.type === 'enum') {
      const match = field.options.find((o) => String(o) === raw);
      args[field.name] = match !== undefined ? match : raw;
      continue;
    }
    if (field.type === 'any') {
      try {
        args[field.name] = JSON.parse(raw);
      } catch {
        errors.push(`Parameter '${field.name}' must be valid JSON.`);
      }
      continue;
    }
    args[field.name] = raw;
  }
  return { args, errors };
}

export interface ToolCallHistoryEntry {
  serverName: string;
  toolName: string;
  status: 'Success' | 'Error';
  calledAt: number;
}

// Session-local audit trail: timestamp, server, tool, status —
// never parameters, results, environment values, or keys.
// Newest first, capped at 50 entries.
export function recordToolCall(
  history: ToolCallHistoryEntry[],
  entry: ToolCallHistoryEntry,
): ToolCallHistoryEntry[] {
  return [entry, ...history].slice(0, 50);
}
