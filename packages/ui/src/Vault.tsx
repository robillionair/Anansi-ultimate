import { useState } from 'react';
import { nativeVaultCommand, type Client, type VaultCommandName } from '@anansi/contracts';

export function Vault({
  workspaceId,
  client = null,
  native = true,
  onRestored,
}: {
  workspaceId: string;
  client?: Client | null;
  native?: boolean;
  onRestored?: () => void;
}) {
  const [jobId, setJobId] = useState('');
  const [artifactId, setArtifactId] = useState('');
  const [sha256, setSha256] = useState('');
  const [archiveId, setArchiveId] = useState('');
  const [archiveSha256, setArchiveSha256] = useState('');
  const [restoreName, setRestoreName] = useState('');
  const [encryptedExport, setEncryptedExport] = useState(false);
  const [useRecovery, setUseRecovery] = useState(false);
  const [previewSummary, setPreviewSummary] = useState('');
  const [restoredId, setRestoredId] = useState('');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const [error, setError] = useState('');

  async function callVault(command: VaultCommandName, request: Record<string, unknown>): Promise<Record<string, unknown>> {
    if (client?.vaultCommand) {
      return await client.vaultCommand(command, request);
    }
    return (await nativeVaultCommand(command, request)) as Record<string, unknown>;
  }

  async function run(label: string, action: () => Promise<unknown>) {
    setBusy(true);
    setError('');
    setNotice('');
    try {
      await action();
      setNotice(label);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Vault operation failed.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="card" aria-labelledby="vault-title" aria-busy={busy}>
      <h2 id="vault-title">Sealed vault</h2>
      <p>
        Credentials and recovery acknowledgement use trusted Windows dialogs. Leave their prefilled username unchanged.
        Enter your vault passphrase in Password for setup and confirmation; after saving the recovery key, enter that
        recovery key in Password for the recovery check. Keys and session handles never enter this page.
      </p>
      {!native && (
        <p className="help">
          Paired browser requests name this origin and workspace in the local native prompt. Initial vault setup and
          sealing are performed in the Windows desktop application.
        </p>
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {notice && (
        <p className="notice" role="status">
          {notice}
        </p>
      )}
      <div className="actions">
        {native && (
          <button
            disabled={busy}
            onClick={() =>
              void run('Recovery acknowledged. Select the verified backup below to seal this workspace.', () =>
                callVault('setup', { workspaceId })
              )
            }
          >
            Set up vault
          </button>
        )}
        <button
          disabled={busy}
          onClick={() =>
            void run(
              native
                ? 'Vault unlocked for this desktop process.'
                : 'Vault unlocked for this paired browser session.',
              () => callVault('unlock', { workspaceId })
            )
          }
        >
          Unlock
        </button>
        <button
          disabled={busy}
          className="secondary"
          onClick={() =>
            void run(
              native
                ? 'Vault locked and the desktop session was revoked.'
                : 'Vault locked and all active sessions were revoked.',
              () => callVault('lock', { workspaceId })
            )
          }
        >
          Lock
        </button>
        <button
          disabled={busy}
          className="secondary"
          onClick={() =>
            void run('Authenticated vault read completed.', () => callVault('read', { workspaceId }))
          }
        >
          Verify authorized read
        </button>
      </div>
      {native && (
        <details>
          <summary>Seal using a verified plaintext backup</summary>
          <p>
            The retained backup remains plaintext by design. Sealing excludes active plaintext search entries but cannot
            securely erase historical copies.
          </p>
          <label htmlFor="vault-job">Backup job ID</label>
          <input id="vault-job" value={jobId} onChange={e => setJobId(e.target.value)} autoComplete="off" />
          <label htmlFor="vault-artifact">Artifact ID</label>
          <input
            id="vault-artifact"
            value={artifactId}
            onChange={e => setArtifactId(e.target.value)}
            autoComplete="off"
          />
          <label htmlFor="vault-hash">Backup SHA-256</label>
          <input
            id="vault-hash"
            value={sha256}
            onChange={e => setSha256(e.target.value)}
            autoComplete="off"
            spellCheck={false}
          />
          <button
            disabled={busy || !jobId || !artifactId || !/^[a-f0-9]{64}$/.test(sha256)}
            onClick={() =>
              void run('Workspace sealed. Unlock it to read encrypted records.', () =>
                callVault('seal', { workspaceId, jobId, artifactId, sha256 })
              )
            }
          >
            Seal workspace
          </button>
        </details>
      )}
      <details>
        <summary>Encrypted vault backup, export, and fresh restore</summary>
        <p>
          Ciphertext backups copy only encrypted envelopes and wrapped keyslots. Encrypted exports rewrap the archive
          under a passphrase entered in the native prompt. Fresh restore authenticates every envelope and remaps IDs
          into a new sealed workspace.
        </p>
        <div className="actions">
          <button
            disabled={busy}
            type="button"
            onClick={() =>
              void run('Ciphertext vault backup created.', async () => {
                const res = await callVault('backup', { workspaceId });
                setArchiveId(String(res.archiveId ?? ''));
                setArchiveSha256(String(res.sha256 ?? ''));
                setEncryptedExport(false);
              })
            }
          >
            Create ciphertext backup
          </button>
          <button
            disabled={busy}
            type="button"
            className="secondary"
            onClick={() =>
              void run('Encrypted vault export created.', async () => {
                const res = await callVault('export', { workspaceId });
                setArchiveId(String(res.archiveId ?? ''));
                setArchiveSha256(String(res.sha256 ?? ''));
                setEncryptedExport(true);
              })
            }
          >
            Create encrypted export
          </button>
          <button
            disabled={busy || !archiveId}
            type="button"
            className="secondary"
            onClick={() =>
              void run('Locked ciphertext backup structure verified.', async () => {
                const res = await callVault('preview', { workspaceId, archiveId });
                setPreviewSummary(
                  `Archive ${String(res.archiveId)}: ${String(res.files)} encrypted files (${String(res.bytes)} bytes), authenticated=${String(res.authenticated)}`
                );
              })
            }
          >
            Preview ciphertext backup
          </button>
        </div>
        {previewSummary && <p className="vault-preview-summary">{previewSummary}</p>}
        <label htmlFor="vault-archive-id">Encrypted archive ID</label>
        <input
          id="vault-archive-id"
          value={archiveId}
          onChange={e => setArchiveId(e.target.value)}
          autoComplete="off"
        />
        <label htmlFor="vault-archive-hash">Expected archive SHA-256</label>
        <input
          id="vault-archive-hash"
          value={archiveSha256}
          onChange={e => setArchiveSha256(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        <label htmlFor="vault-restore-name">New restored workspace name</label>
        <input
          id="vault-restore-name"
          value={restoreName}
          onChange={e => setRestoreName(e.target.value)}
          autoComplete="off"
          maxLength={64}
        />
        <label>
          <input
            type="checkbox"
            checked={encryptedExport}
            onChange={e => {
              setEncryptedExport(e.target.checked);
              if (e.target.checked) setUseRecovery(false);
            }}
          />{' '}
          Archive is an outer-encrypted export
        </label>
        <label>
          <input
            type="checkbox"
            checked={useRecovery}
            disabled={encryptedExport}
            onChange={e => setUseRecovery(e.target.checked)}
          />{' '}
          Authenticate source using recovery key
        </label>
        <button
          disabled={busy || !archiveId || !restoreName.trim()}
          type="button"
          onClick={() =>
            void run('Encrypted archive restored into a fresh sealed workspace.', async () => {
              const res = await callVault('restore', {
                workspaceId,
                archiveId,
                name: restoreName.trim(),
                encryptedExport,
                useRecovery,
                ...(archiveSha256 ? { expectedSha256: archiveSha256 } : {}),
              });
              setRestoredId(String(res.restoredWorkspaceId ?? ''));
            })
          }
        >
          Restore to new sealed workspace
        </button>
        {restoredId && (
          <p className="vault-restored-workspace">
            Restored workspace ID: <code>{restoredId}</code>
          </p>
        )}
      </details>
    </section>
  );
}
