import { useState } from 'react';
import {
  compileApprovedContext,
  type GhostDeletedFragment,
} from './ghostPrompt.ts';

/**
 * M11 GHOST-PROMPT Slice 3: dual-panel comparison (§58).
 *
 * Panel A is the final prompt exactly; Panel B is the final prompt plus
 * exactly the user-selected fragments, compiled by
 * compileApprovedContext. Unselected text appears nowhere in Panel B.
 * Counts are labeled units: Unicode scalar characters, estimated tokens
 * (chars / 4, rounded up), and a simulated cost delta with its
 * assumption stated. Responses are deterministic offline reference texts
 * unless the user explicitly clicks "Send both to Ask ANANSI", which
 * dispatches exactly the two panel prompts through the user's own Ask
 * settings. Nothing here auto-submits, auto-rewrites, or sends anything
 * on mount or on selection.
 */

export const GHOST_SIMULATED_COST_PER_1M_TOKENS = 2.0;

export function ghostCharCount(text: string): number {
  return Array.from(text).length;
}

export function ghostEstimateTokens(chars: number): number {
  return Math.ceil(chars / 4);
}

export function ghostReferenceResponse(prompt: string): string {
  return `Reference response (offline, not a model answer) to: ${prompt}`;
}

export type GhostComparisonDispatch = (
  promptA: string,
  promptB: string,
) => Promise<{ answerA: string; answerB: string }>;

/** Run two single-shot asks in order and return both answers. */
export async function runComparisonDispatch(
  ask: (prompt: string) => Promise<string>,
  promptA: string,
  promptB: string,
): Promise<{ answerA: string; answerB: string }> {
  const answerA = await ask(promptA);
  const answerB = await ask(promptB);
  return { answerA, answerB };
}

export function GhostPromptCompare({
  finalText,
  selectedFragments,
  onCombinePrompt,
  dispatch,
}: {
  finalText: string;
  selectedFragments: GhostDeletedFragment[];
  onCombinePrompt?: (text: string) => void;
  dispatch?: GhostComparisonDispatch;
}) {
  const promptB = compileApprovedContext(finalText, selectedFragments);
  const charsA = ghostCharCount(finalText);
  const charsB = ghostCharCount(promptB);
  const tokensA = ghostEstimateTokens(charsA);
  const tokensB = ghostEstimateTokens(charsB);
  const costDelta =
    ((tokensB - tokensA) * GHOST_SIMULATED_COST_PER_1M_TOKENS) / 1_000_000;
  const [feedback, setFeedback] = useState<string | null>(null);
  const [dispatched, setDispatched] = useState<{
    answerA: string;
    answerB: string;
  } | null>(null);
  const [sending, setSending] = useState(false);
  const [dispatchError, setDispatchError] = useState('');
  const answerA = dispatched?.answerA ?? ghostReferenceResponse(finalText);
  const answerB = dispatched?.answerB ?? ghostReferenceResponse(promptB);

  return (
    <section aria-label="GHOST-PROMPT comparison">
      <h4>Compare responses</h4>
      <p>
        Comparison responses are reference texts, not model answers, unless
        you send them to Ask ANANSI. They are a comparison, not evidence
        of subconscious insight.
      </p>
      <section aria-label="Final Prompt Only">
        <h5>Final Prompt Only</h5>
        <p>{finalText === '' ? '(empty)' : finalText}</p>
        <p>
          Characters: {charsA}; estimated tokens: {tokensA}
        </p>
        <p>{answerA}</p>
      </section>
      <section aria-label="Final Prompt + Approved Draft Context">
        <h5>Final Prompt + Approved Draft Context</h5>
        <p>{promptB === '' ? '(empty)' : promptB}</p>
        <p>
          Characters: {charsB}; estimated tokens: {tokensB}
        </p>
        <p>Exactly what extra context was included:</p>
        <ul>
          {selectedFragments.map((fragment) => (
            <li key={fragment.seq}>
              {fragment.text} (deleted at offset {fragment.offset})
            </li>
          ))}
        </ul>
        <p>{answerB}</p>
      </section>
      <p>
        Extra characters: {charsB - charsA}; extra estimated tokens:{' '}
        {tokensB - tokensA}; simulated cost delta: ${costDelta.toFixed(6)}{' '}
        (Simulated — assumes ${GHOST_SIMULATED_COST_PER_1M_TOKENS.toFixed(2)}{' '}
        per 1M tokens).
      </p>
      {dispatch && (
        <div>
          <button
            type="button"
            disabled={sending}
            onClick={async () => {
              setSending(true);
              setDispatchError('');
              try {
                const result = await dispatch(finalText, promptB);
                setDispatched(result);
              } catch (error) {
                setDispatchError(
                  error instanceof Error ? error.message : 'Dispatch failed.',
                );
              } finally {
                setSending(false);
              }
            }}
          >
            {sending ? 'Sending…' : 'Send both to Ask ANANSI'}
          </button>
          <p>Uses your configured endpoint and model.</p>
          {dispatchError !== '' && <p>{dispatchError}</p>}
          {dispatched && <p>Answers from Ask ANANSI.</p>}
        </div>
      )}
      <div>
        <p>Which answer was more useful?</p>
        <button type="button" onClick={() => setFeedback('Final Prompt Only')}>
          Final Prompt Only
        </button>
        <button
          type="button"
          onClick={() => setFeedback('Final Prompt + Draft Context')}
        >
          Final Prompt + Draft Context
        </button>
        <button type="button" onClick={() => setFeedback('Neither')}>
          Neither
        </button>
        {feedback !== null && <p>You found {feedback} more useful.</p>}
      </div>
      {onCombinePrompt && (
        <button type="button" onClick={() => onCombinePrompt(promptB)}>
          Combine into an updated prompt
        </button>
      )}
    </section>
  );
}
