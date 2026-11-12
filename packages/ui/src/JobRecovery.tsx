import type { Job } from '@anansi/contracts';

/// The recovery notice shown for a Queued job whose storage root
/// exhausted claim recovery. Shared by the Jobs list and the
/// Documents import progress section so the message, the
/// attempt count, and the Retry recovery affordance cannot
/// drift between views. Renders nothing without a diagnostic;
/// the button appears only while the job is still Queued.
export function JobRecoveryNotice({
  diagnostic,
  state,
  busy,
  onRecover,
}: {
  diagnostic: Job['recoveryDiagnostic'];
  state: Job['state'];
  busy: boolean;
  onRecover: () => void;
}) {
  if (!diagnostic) return null;
  return (
    <>
      <p role="alert" className="error">
        {diagnostic.message} Attempts used: {diagnostic.attempts}.
      </p>
      {state === 'Queued' && (
        <div className="actions">
          <button disabled={busy} onClick={onRecover}>
            Retry recovery
          </button>
        </div>
      )}
    </>
  );
}
