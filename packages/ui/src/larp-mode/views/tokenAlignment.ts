// LARP Mode Slice 4: pure token alignment for the attention heatmap.
// A cell is true when the raw word equals the compiled word after the
// same normalization (lowercase, end punctuation stripped). This is a
// genuine word-presence matrix — NOT attention weights, which do not
// exist in this app, and NOT provider tokens, which this app cannot
// produce (it has no tokenizer).

import { tokenizeWords } from '../telemetryProducers.ts';

export const MAX_HEATMAP_ROWS = 32;
export const MAX_HEATMAP_COLS = 64;

export interface AlignmentMatrix {
  rows: string[];
  cols: string[];
  matches: boolean[][];
  matchCount: number;
}

export function alignmentMatrix(rawTokens: string[], compiledTokens: string[]): AlignmentMatrix {
  const rows = tokenizeWords(rawTokens.join(' ')).slice(0, MAX_HEATMAP_ROWS);
  const cols = tokenizeWords(compiledTokens.join(' ')).slice(0, MAX_HEATMAP_COLS);
  let matchCount = 0;
  const matches = rows.map((raw) =>
    cols.map((compiled) => {
      const hit = raw === compiled;
      if (hit) matchCount += 1;
      return hit;
    }),
  );
  return { rows, cols, matches, matchCount };
}

export function latestAskTokens(
  events: { kind: string; rawTokens?: string[]; compiledTokens?: string[] }[],
): { rawTokens: string[]; compiledTokens: string[] } | null {
  for (let i = events.length - 1; i >= 0; i--) {
    const event = events[i];
    if (event.kind === 'ask-tokens') {
      return {
        rawTokens: event.rawTokens ?? [],
        compiledTokens: event.compiledTokens ?? [],
      };
    }
  }
  return null;
}

export { tokenizeWords };
