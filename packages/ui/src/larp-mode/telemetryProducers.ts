// LARP Mode Slice 2: live telemetry producers. Both functions push only
// real, measured-or-server-reported figures — never invented tokens:
//
// - Ask runs: the server manifest's estimatedTokens IS the compiled
//   prompt's estimate (crates/core/src/ask.rs: compiled_prompt.len()/4).
//   The raw input estimate uses the same /4 estimator on the prompt the
//   user actually typed. Output has no compilation step in Ask, so the
//   answer estimate goes in both output fields, documented here. All
//   Ask figures carry estimated: true, so the delta stream badges them.
//   A run completing with insufficientEvidence still ran the context
//   compiler, so it fires a compilation event — but no token-delta,
//   because a compiled estimate of 0 would fake a -100% compression.
// - Memory loads: nodes mapped from real MemoryMetadata records
//   (kind 'note' stays a note, everything else reads as memory).
//
// Both are no-ops on empty input, and both take an injectable stream so
// unit tests never touch the process singleton. Call sites wrap these in
// try/catch: a dropped telemetry event is acceptable, a broken Ask run
// or Memory load is not.

import type { MemoryMetadata } from '@anansi/contracts';
import {
  telemetryStream,
  type AnansiTelemetryStream,
  type MemoryNode,
} from './telemetry.ts';

export interface AskRunResponseLike {
  status: string;
  answer: string;
  manifest: { estimatedTokens: number };
}

export function estimateTokens(chars: number): number {
  return Math.ceil(Math.max(0, chars) / 4);
}

export function reportAskRun(
  prompt: string,
  response: AskRunResponseLike | null,
  latencyMs: number,
  stream: AnansiTelemetryStream = telemetryStream,
): void {
  if (!response) return;
  const at = Date.now();
  stream.push({ kind: 'compilation', at, label: 'ask-run-completed' });
  if (response.status !== 'answered') return;
  const compiled = response.manifest.estimatedTokens;
  if (!(compiled > 0)) return;
  const answerEstimate = estimateTokens(response.answer.length);
  stream.push({
    kind: 'token-delta',
    at,
    rawInputTokens: estimateTokens(prompt.length),
    compiledInputTokens: compiled,
    rawOutputTokens: answerEstimate,
    compiledOutputTokens: answerEstimate,
    latencyMs: Math.max(0, Math.round(latencyMs)),
    estimated: true,
  });
}

export function reportMemorySnapshot(
  metadatas: MemoryMetadata[],
  stream: AnansiTelemetryStream = telemetryStream,
): void {
  const nodes: MemoryNode[] = metadatas.map((m) => ({
    id: m.id,
    kind: m.kind === 'note' ? 'note' : 'memory',
    title: m.title,
  }));
  if (nodes.length === 0) return;
  stream.push({ kind: 'memory', at: Date.now(), nodes });
}
