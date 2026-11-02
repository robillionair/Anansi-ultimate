import { useState } from 'react';
import type { GhostSession, GhostDeletedFragment, GhostPauseEvent } from './ghostPrompt.ts';
import { GhostPrompt3D } from './GhostPrompt3D.tsx';

export const GHOST_CONSENT_TEXT =
  'When enabled, ANANSI can temporarily track revisions made in this editor. Deleted text remains available for your review until you discard it or explicitly save it.';

export function GhostFragmentsTable({
  currentText,
  fragments,
  pauses,
  insertedChars,
  editCount,
}: {
  currentText: string;
  fragments: GhostDeletedFragment[];
  pauses: GhostPauseEvent[];
  insertedChars: number;
  editCount: number;
}) {
  return (
    <section aria-label="Retained & Deleted Fragments">
      <h3>Retained &amp; Deleted Fragments</h3>
      <p>
        Retained text: {currentText === '' ? '(empty)' : currentText}
      </p>
      <table aria-label="Deleted fragments timeline">
        <thead>
          <tr>
            <th>Fragment</th>
            <th>Offset</th>
            <th>Hash</th>
          </tr>
        </thead>
        <tbody>
          {fragments.map((fragment) => (
            <tr key={fragment.seq}>
              <td>
                <s>{fragment.text}</s> <span>deleted</span>
              </td>
              <td>{fragment.offset}</td>
              <td>{fragment.hash}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {fragments.length === 0 && <p>No deleted fragments recorded.</p>}
      <h4>Pause events</h4>
      {pauses.length === 0 ? (
        <p>No pause events recorded.</p>
      ) : (
        <ul>
          {pauses.map((pause, index) => (
            <li key={index}>
              Pause of {pause.durationMs}ms at {pause.elapsedMs}ms
            </li>
          ))}
        </ul>
      )}
      <p>
        Writing cadence: {insertedChars} characters across {editCount} edit events.
      </p>
    </section>
  );
}

export function GhostPromptStrip({
  engine,
  onEnable,
  onPause,
  onResume,
  onClear,
  onStop,
}: {
  engine: GhostSession;
  onEnable: () => void;
  onPause: () => void;
  onResume: () => void;
  onClear: () => void;
  onStop: () => void;
}) {
  const [showConsent, setShowConsent] = useState(false);
  const [showReview, setShowReview] = useState(false);
  const [show3D, setShow3D] = useState(false);
  const [purged, setPurged] = useState(false);
  const isOn = engine.state === 'recording' || engine.state === 'paused';
  const fragments = engine.deletedFragments();
  const cadence = engine.cadence();
  // The notice refers to the current, empty buffer: once new edits land
  // after a clear, it stops applying and hides itself.
  const showPurged =
    purged && engine.events.length === 0 && engine.pauses.length === 0;

  if (!isOn) {
    return (
      <section aria-label="GHOST-PROMPT controls">
        <button type="button" onClick={() => setShowConsent(true)}>
          Enable Draft Exploration
        </button>
        {showPurged && <p>Draft history purged.</p>}
        {showConsent && (
          <div role="dialog" aria-label="Enable Draft Exploration consent">
            <p>{GHOST_CONSENT_TEXT}</p>
            <p>
              Recording is in-memory only. Nothing is sent to any provider,
              and drafts are never saved unless you explicitly save them.
            </p>
            <button
              type="button"
              onClick={() => {
                setShowConsent(false);
                setPurged(false);
                onEnable();
              }}
            >
              Enable
            </button>
            <button type="button" onClick={() => setShowConsent(false)}>
              Cancel
            </button>
          </div>
        )}
      </section>
    );
  }

  return (
    <section aria-label="GHOST-PROMPT controls">
      <p>
        <strong>Draft Exploration ON</strong>{' '}
        <span>{engine.state === 'paused' ? 'Paused' : 'Recording'}</span>
      </p>
      <div>
        {engine.state === 'recording' ? (
          <button type="button" onClick={onPause}>
            Pause recording
          </button>
        ) : (
          <button type="button" onClick={onResume}>
            Resume recording
          </button>
        )}
        <button type="button" onClick={() => setShowReview((value) => !value)}>
          Review drafts
        </button>
        <button
          type="button"
          onClick={() => {
            onClear();
            setPurged(true);
          }}
        >
          Clear draft history
        </button>
        <button
          type="button"
          onClick={() => {
            onStop();
            setShowReview(false);
            setPurged(true);
          }}
        >
          Stop &amp; discard session
        </button>
      </div>
      {showPurged && <p>Draft history purged.</p>}
      {showReview && (
        <div>
          <GhostFragmentsTable
            currentText={engine.currentText}
            fragments={fragments}
            pauses={engine.pauses}
            insertedChars={cadence.insertedChars}
            editCount={cadence.editCount}
          />
          <button type="button" onClick={() => setShow3D((value) => !value)}>
            {show3D ? 'Collapse 3D view' : 'Expand 3D view'}
          </button>
          {show3D && (
            <GhostPrompt3D
              snapshot={{
                currentText: engine.currentText,
                fragments,
                pauses: engine.pauses,
                events: engine.events,
              }}
            />
          )}
        </div>
      )}
    </section>
  );
}
