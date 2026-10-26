import { useEffect, useRef, useState } from 'react';
import type {
  ArtifactRecord,
  Client,
  ForgePollResponse,
  ForgeSpecApprovalStatusResponse,
  ProductSpecDoc,
  SourcePackageDoc,
} from '@anansi/contracts';
import type { DisclosureRecord } from '@anansi/contracts';
import { DisclosurePanel } from './Providers';
import { ForgeRunController } from './forgeRun';

// ---------------------------------------------------------------------------
// Product Forge (M7 Slice 1, MASTER_SPEC §§28–33): saved
// concept → product specification → development plan →
// source package → export-for-review. There is no
// execution sandbox in this slice (§32), so build, test,
// preview and deployment are shown as unavailable and the
// generated source is only ever displayed as inert text
// and exported for review — never run by ANANSI.

export function parseProductSpecBody(body: string): ProductSpecDoc | null {
  try {
    const parsed = JSON.parse(body) as ProductSpecDoc;
    return parsed && parsed.format === 'anansi-product-spec' && parsed.spec
      ? parsed
      : null;
  } catch {
    return null;
  }
}

export function parseSourcePackageBody(body: string): SourcePackageDoc | null {
  try {
    const parsed = JSON.parse(body) as SourcePackageDoc;
    return parsed && parsed.format === 'anansi-source-package' && Array.isArray(parsed.files)
      ? parsed
      : null;
  } catch {
    return null;
  }
}

const UNAVAILABLE_STAGES: Array<[string, string]> = [
  ['Build', 'Unavailable — no execution sandbox exists in this slice (§32); the source is exported for review instead. ANANSI has not built this code.'],
  ['Tests', 'Unavailable — no execution sandbox exists in this slice (§32). ANANSI has not run any tests on this code.'],
  ['Preview', 'Unavailable — generated HTML is never rendered or executed by ANANSI in this slice.'],
  ['Deployment', 'Unavailable — this prototype is not deployed anywhere; it is a local source package.'],
];

export function PipelineStages(props: {
  savedStages?: string[];
  generatedStages?: string[];
}) {
  const savedStages = props.savedStages ?? [];
  const generatedStages = props.generatedStages ?? [];
  return (
    <div>
      <h5>Pipeline (§30)</h5>
      <ul aria-label="Forge pipeline stages">
        {savedStages.map(stage => (
          <li key={stage}>{stage}: Completed — generated and saved as a project artifact.</li>
        ))}
        {generatedStages.map(stage => (
          <li key={stage}>{stage}: Generated — not yet saved as a project artifact.</li>
        ))}
        {UNAVAILABLE_STAGES.map(([stage, reason]) => (
          <li key={stage}>{stage}: {reason}</li>
        ))}
      </ul>
    </div>
  );
}

export function SpecificationView({ doc }: { doc: ProductSpecDoc }) {
  const conceptName =
    (doc.concept as { name?: string } | null)?.name ?? 'Concept';
  return (
    <div>
      <h4>Product specification: {doc.spec.title}</h4>
      <p className="help">
        Prototype template: dependency-free static web application. Drafted from concept “{conceptName}”
        in artifact {doc.sourceArtifactId} version {doc.sourceVersion}. A specification is a plan,
        not evidence the product exists or works.
      </p>
      <p>{doc.spec.summary}</p>
      <p><strong>Purpose:</strong> {doc.spec.purpose}</p>
      <p><strong>Intended users:</strong> {doc.spec.intendedUsers}</p>
      <h5>Features</h5>
      <ul>{doc.spec.features.map((f, i) => <li key={i}>{f}</li>)}</ul>
      <h5>Pages</h5>
      <ul>{doc.spec.pages.map((p, i) => <li key={i}>{p}</li>)}</ul>
      {doc.spec.nonGoals.length > 0 && (
        <>
          <h5>Non-goals</h5>
          <ul>{doc.spec.nonGoals.map((n, i) => <li key={i}>{n}</li>)}</ul>
        </>
      )}
      {doc.spec.dataAndState && <p><strong>Data and state:</strong> {doc.spec.dataAndState}</p>}
      <h5>Acceptance criteria (for a person to check)</h5>
      <ul>{doc.spec.acceptanceCriteria.map((c, i) => <li key={i}>{c}</li>)}</ul>
      {doc.spec.openQuestions.length > 0 && (
        <>
          <h5>Open questions</h5>
          <ul>{doc.spec.openQuestions.map((q, i) => <li key={i}>{q}</li>)}</ul>
        </>
      )}
    </div>
  );
}

export function SourcePackageView(props: {
  doc: SourcePackageDoc;
  artifact?: ArtifactRecord;
  client?: Client;
  workspaceId?: string;
  notify?: (message: string, isError: boolean) => void;
}) {
  const { doc } = props;
  const [selected, setSelected] = useState(0);
  const [exporting, setExporting] = useState(false);
  const file = doc.files[Math.min(selected, doc.files.length - 1)];
  async function exportPackage() {
    if (!props.client || !props.workspaceId || !props.artifact || !props.notify) return;
    setExporting(true);
    try {
      const result = await props.client.exportForgePackage(
        props.workspaceId,
        props.artifact.artifactId,
        props.artifact.version,
      );
      const bytes = Uint8Array.from(atob(result.tarBase64), c => c.charCodeAt(0));
      const url = URL.createObjectURL(new Blob([bytes], { type: 'application/x-tar' }));
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = result.filename;
      anchor.click();
      URL.revokeObjectURL(url);
      props.notify(
        `Exported ${result.filename} (SHA-256 ${result.sha256.slice(0, 12)}…). Review the files before any use; ANANSI does not run them.`,
        false,
      );
    } catch (e) {
      props.notify(e instanceof Error ? e.message : 'Export failed.', true);
    } finally {
      setExporting(false);
    }
  }
  return (
    <div>
      <h4>Source package: {doc.conceptName}</h4>
      <p className="help">
        Generated from specification artifact {doc.specArtifactId} version {doc.specVersion}
        (itself drafted from artifact {doc.sourceArtifactId} version {doc.sourceVersion}).
        Prototype — not a finished product.
      </p>
      <h5>Development plan</h5>
      <p>{doc.plan.summary}</p>
      <ol>{doc.plan.steps.map((s, i) => <li key={i}>{s}</li>)}</ol>
      <p><strong>Build notes:</strong> {doc.plan.buildNotes}</p>
      <p><strong>Testing notes:</strong> {doc.plan.testingNotes}</p>
      <h5>Files (inert text — ANANSI does not run or preview generated code)</h5>
      <ul aria-label="Package files">
        {doc.files.map((f, i) => (
          <li key={f.path}>
            <button type="button" className="secondary" onClick={() => setSelected(i)}>
              {f.path}
            </button>
            <span> · {f.content.length} characters</span>
          </li>
        ))}
      </ul>
      {file && (
        <div>
          <h6>{file.path}</h6>
          <pre className="provider-output">{file.content}</pre>
        </div>
      )}
      <h5>Limitations</h5>
      <ul>{doc.limitations.map((l, i) => <li key={i}>{l}</li>)}</ul>
      {props.artifact ? (
        <PipelineStages savedStages={['Product specification', 'Development plan', 'Source generation']} />
      ) : (
        <PipelineStages generatedStages={['Product specification', 'Development plan', 'Source generation']} />
      )}
      {props.artifact && props.client && (
        <button type="button" disabled={exporting} onClick={() => void exportPackage()}>
          Export source package (.tar)
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------

type Notify = (message: string, isError: boolean) => void;

function useForgePolling(
  controller: () => ForgeRunController,
  run: ForgePollResponse | null,
  setRun: (run: ForgePollResponse) => void,
  notify: Notify,
) {
  useEffect(() => {
    if (!run || (run.status !== 'running' && run.status !== 'cancelling')) return;
    const timer = setInterval(() => {
      void (async () => {
        try {
          const polled = await controller().pollOnce();
          if (polled) setRun(polled);
        } catch (e) {
          notify(e instanceof Error ? e.message : 'Polling failed.', true);
        }
      })();
    }, 600);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [run?.runId, run?.status]);
}

export function ForgeSpecPanel(props: {
  client: Client;
  workspaceId: string;
  projectId: string;
  source: ArtifactRecord;
  concepts: Array<{ name: string }>;
  notify: Notify;
  onSaved: () => void;
}) {
  const { client, workspaceId, source } = props;
  const [conceptIndex, setConceptIndex] = useState(0);
  const [purpose, setPurpose] = useState('');
  const [users, setUsers] = useState('');
  const [mustHave, setMustHave] = useState('');
  const [lock, setLock] = useState(true);
  const [run, setRun] = useState<ForgePollResponse | null>(null);
  const [disclosure, setDisclosure] = useState<DisclosureRecord | null>(null);
  const [savedId, setSavedId] = useState('');
  const controllerRef = useRef<ForgeRunController | null>(null);
  function controller(): ForgeRunController {
    if (!controllerRef.current) {
      controllerRef.current = new ForgeRunController(client, workspaceId, 'spec');
    }
    return controllerRef.current;
  }
  useEffect(() => () => {
    void controllerRef.current?.reset();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source.artifactId, source.version]);
  useForgePolling(controller, run, setRun, props.notify);
  const running = run?.status === 'running' || run?.status === 'cancelling';
  async function start(e: React.FormEvent) {
    e.preventDefault();
    setSavedId('');
    setDisclosure(null);
    try {
      const started = await controller().start({
        projectId: props.projectId,
        sourceArtifactId: source.artifactId,
        sourceVersion: source.version,
        conceptIndex,
        purpose,
        intendedUsers: users,
        mustHave,
        zeroCostLock: lock,
      });
      if (started) {
        if (started.disclosure) setDisclosure(started.disclosure);
        setRun({
          contractVersion: 1,
          runId: started.runId,
          status: 'running',
          partialChars: 0,
        });
      }
    } catch (err) {
      props.notify(err instanceof Error ? err.message : 'Could not start Product Forge.', true);
    }
  }
  async function save() {
    if (!run) return;
    try {
      const saved = await client.saveForgeSpec(workspaceId, run.runId);
      setSavedId(saved.artifact.artifactId);
      props.notify(
        saved.alreadySaved
          ? 'This specification was already saved as an artifact.'
          : `Specification saved as artifact ${saved.artifact.artifactId} (version ${saved.artifact.version}).`,
        false,
      );
      props.onSaved();
    } catch (err) {
      props.notify(err instanceof Error ? err.message : 'Could not save the specification.', true);
    }
  }
  return (
    <div>
      <h4>Product Forge — develop a concept (§§28–33)</h4>
      <p className="help">
        Forge drafts a product specification for one concept from this set, pinned to this artifact
        version ({source.version}). Template: a small dependency-free static web application —
        a prototype, not SaaS: no authentication, database or deployment. Nothing is generated
        until you submit. Saving stores the draft; generating source requires explicitly
        approving the saved specification version afterwards — saving or revising a
        specification does not approve it.
      </p>
      <form onSubmit={e => void start(e)}>
        <label htmlFor="forge-concept">Concept to develop</label>
        <select
          id="forge-concept"
          value={conceptIndex}
          onChange={e => setConceptIndex(Number(e.target.value))}
        >
          {props.concepts.map((c, i) => (
            <option key={i} value={i}>{c.name}</option>
          ))}
        </select>
        <label htmlFor="forge-purpose">Purpose — what should it do?</label>
        <textarea id="forge-purpose" rows={2} value={purpose} onChange={e => setPurpose(e.target.value)} required />
        <label htmlFor="forge-users">Intended users</label>
        <textarea id="forge-users" rows={2} value={users} onChange={e => setUsers(e.target.value)} required />
        <label htmlFor="forge-must-have">Must-have behavior</label>
        <textarea id="forge-must-have" rows={2} value={mustHave} onChange={e => setMustHave(e.target.value)} required />
        <label>
          <input type="checkbox" checked={lock} onChange={e => setLock(e.target.checked)} /> Zero-Cost Lock (never spend)
        </label>
        <button type="submit" disabled={running}>Generate product specification</button>
        {running && (
          <button
            type="button"
            className="secondary"
            disabled={run?.status === 'cancelling'}
            onClick={() => void (async () => {
              const cancelled = await controller().cancel();
              if (cancelled) setRun(cancelled);
            })()}
          >
            Cancel
          </button>
        )}
      </form>
      {run?.status === 'running' && <p role="status">Drafting specification… {run.partialChars} characters received.</p>}
      {run?.status === 'cancelling' && <p role="status">Cancelling… waiting for the provider stream to stop.</p>}
      {run?.status === 'cancelled' && <p role="status">Specification drafting cancelled. Nothing was saved.</p>}
      {run && (run.status === 'failed' || run.status === 'invalid') && (
        <p role="alert">Specification {run.status}: {run.error ?? 'the provider did not produce a usable specification.'} Nothing was saved; you can try again.</p>
      )}
      {disclosure && <DisclosurePanel disclosure={disclosure} />}
      {run?.spec && (
        <div>
          <SpecificationView doc={run.spec} />
          {savedId ? (
            <p role="status">
              Specification draft saved as artifact {savedId}. Open it from the Artifacts
              list to approve this version, then generate the development plan and source package.
            </p>
          ) : (
            <button type="button" onClick={() => void save()}>
              Save specification draft
            </button>
          )}
        </div>
      )}
    </div>
  );
}

export function ForgeBuildPanel(props: {
  client: Client;
  workspaceId: string;
  projectId: string;
  spec: ArtifactRecord;
  notify: Notify;
  onSaved: () => void;
}) {
  const { client, workspaceId, spec } = props;
  const [lock, setLock] = useState(true);
  const [run, setRun] = useState<ForgePollResponse | null>(null);
  const [disclosure, setDisclosure] = useState<DisclosureRecord | null>(null);
  const [savedId, setSavedId] = useState('');
  const [savedArtifact, setSavedArtifact] = useState<ArtifactRecord | null>(null);
  const [approval, setApproval] = useState<ForgeSpecApprovalStatusResponse | null>(null);
  const [approving, setApproving] = useState(false);
  const controllerRef = useRef<ForgeRunController | null>(null);
  function controller(): ForgeRunController {
    if (!controllerRef.current) {
      controllerRef.current = new ForgeRunController(client, workspaceId, 'build');
    }
    return controllerRef.current;
  }
  useEffect(() => () => {
    void controllerRef.current?.reset();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [spec.artifactId, spec.version]);
  // Approval is backend state bound to this exact version:
  // it is loaded per version, and only the explicit approve
  // action below creates it.
  useEffect(() => {
    setApproval(null);
    void (async () => {
      try {
        setApproval(await client.specApprovalStatus(workspaceId, {
          projectId: props.projectId,
          specArtifactId: spec.artifactId,
          specVersion: spec.version,
        }));
      } catch (e) {
        props.notify(e instanceof Error ? e.message : 'Could not load specification approval status.', true);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [spec.artifactId, spec.version]);
  async function approve() {
    setApproving(true);
    try {
      const result = await client.approveForgeSpec(workspaceId, {
        projectId: props.projectId,
        specArtifactId: spec.artifactId,
        specVersion: spec.version,
      });
      setApproval({
        contractVersion: 1,
        approved: true,
        approvalArtifactId: result.approvalArtifactId,
        specArtifactId: spec.artifactId,
        specVersion: spec.version,
      });
      props.notify(
        result.alreadyApproved
          ? 'This specification version was already approved.'
          : `Specification version ${spec.version} approved for source generation.`,
        false,
      );
    } catch (e) {
      props.notify(e instanceof Error ? e.message : 'Could not approve this specification version.', true);
    } finally {
      setApproving(false);
    }
  }
  useForgePolling(controller, run, setRun, props.notify);
  const running = run?.status === 'running' || run?.status === 'cancelling';
  async function start() {
    setSavedId('');
    setSavedArtifact(null);
    setDisclosure(null);
    try {
      const started = await controller().start({
        projectId: props.projectId,
        specArtifactId: spec.artifactId,
        specVersion: spec.version,
        zeroCostLock: lock,
      });
      if (started) {
        if (started.disclosure) setDisclosure(started.disclosure);
        setRun({ contractVersion: 1, runId: started.runId, status: 'running', partialChars: 0 });
      }
    } catch (err) {
      props.notify(err instanceof Error ? err.message : 'Could not start source generation.', true);
    }
  }
  async function save() {
    if (!run) return;
    try {
      const saved = await client.saveForgeBuild(workspaceId, run.runId);
      setSavedId(saved.artifact.artifactId);
      setSavedArtifact(saved.artifact);
      props.notify(
        saved.alreadySaved
          ? 'This source package was already saved as an artifact.'
          : `Source package saved as artifact ${saved.artifact.artifactId} (version ${saved.artifact.version}).`,
        false,
      );
      props.onSaved();
    } catch (err) {
      props.notify(err instanceof Error ? err.message : 'Could not save the source package.', true);
    }
  }
  return (
    <div>
      <h4>Product Forge — generate plan &amp; source</h4>
      <p className="help">
        Generates a development plan and the prototype source files from this specification,
        pinned to version {spec.version}. The files are shown as inert text and saved as an
        immutable source-package artifact; ANANSI never runs them (§32 — export is for review).
      </p>
      {approval ? (
        approval.approved ? (
          <p role="status">
            Specification version {spec.version}: approved (approval artifact {approval.approvalArtifactId}).
            Source generation is allowed for this version only.
          </p>
        ) : (
          <p role="status">
            Specification version {spec.version}: not approved. Approve this version explicitly
            before generating source. Saving or revising a specification does not approve it, and
            approval of an earlier version does not carry over to this version.
          </p>
        )
      ) : (
        <p role="status">Loading specification approval status…</p>
      )}
      {!approval?.approved && (
        <button type="button" disabled={approving} onClick={() => void approve()}>
          Approve this specification version
        </button>
      )}
      <label>
        <input type="checkbox" checked={lock} onChange={e => setLock(e.target.checked)} /> Zero-Cost Lock (never spend)
      </label>
      <button type="button" disabled={running || approval?.approved === false} onClick={() => void start()}>
        Generate development plan &amp; source
      </button>
      {running && (
        <button
          type="button"
          className="secondary"
          disabled={run?.status === 'cancelling'}
          onClick={() => void (async () => {
            const cancelled = await controller().cancel();
            if (cancelled) setRun(cancelled);
          })()}
        >
          Cancel
        </button>
      )}
      {run?.status === 'running' && <p role="status">Generating… {run.partialChars} characters received.</p>}
      {run?.status === 'cancelling' && <p role="status">Cancelling… waiting for the provider stream to stop.</p>}
      {run?.status === 'cancelled' && <p role="status">Generation cancelled. Nothing was saved.</p>}
      {run && (run.status === 'failed' || run.status === 'invalid') && (
        <p role="alert">Generation {run.status}: {run.error ?? 'the provider did not produce a usable source package.'} Nothing was saved; you can try again.</p>
      )}
      {disclosure && <DisclosurePanel disclosure={disclosure} />}
      {run?.sourcePackage && (
        <div>
          <SourcePackageView
            doc={run.sourcePackage}
            artifact={savedArtifact ?? undefined}
            client={client}
            workspaceId={workspaceId}
            notify={props.notify}
          />
          {savedId ? (
            <p role="status">Source package saved as artifact {savedId}. Open it from the Artifacts list to inspect and export it.</p>
          ) : (
            <button type="button" onClick={() => void save()}>Save source package as artifact</button>
          )}
        </div>
      )}
    </div>
  );
}

export function ForgeRevisePanel(props: {
  client: Client;
  workspaceId: string;
  projectId: string;
  pkg: ArtifactRecord;
  doc: SourcePackageDoc;
  notify: Notify;
  onSaved: () => void;
}) {
  const { client, workspaceId, pkg, doc } = props;
  const [instructions, setInstructions] = useState('');
  const [lock, setLock] = useState(true);
  const [run, setRun] = useState<ForgePollResponse | null>(null);
  const [disclosure, setDisclosure] = useState<DisclosureRecord | null>(null);
  const [savedVersion, setSavedVersion] = useState(0);
  const [rejected, setRejected] = useState(false);
  const controllerRef = useRef<ForgeRunController | null>(null);
  function controller(): ForgeRunController {
    if (!controllerRef.current) {
      controllerRef.current = new ForgeRunController(client, workspaceId, 'revise');
    }
    return controllerRef.current;
  }
  useEffect(() => () => {
    void controllerRef.current?.reset();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pkg.artifactId, pkg.version]);
  useForgePolling(controller, run, setRun, props.notify);
  const running = run?.status === 'running' || run?.status === 'cancelling';
  async function start(e: React.FormEvent) {
    e.preventDefault();
    setSavedVersion(0);
    setRejected(false);
    setDisclosure(null);
    try {
      const started = await controller().start({
        projectId: props.projectId,
        packageArtifactId: pkg.artifactId,
        packageVersion: pkg.version,
        instructions,
        zeroCostLock: lock,
      });
      if (started) {
        if (started.disclosure) setDisclosure(started.disclosure);
        setRun({ contractVersion: 1, runId: started.runId, status: 'running', partialChars: 0 });
      }
    } catch (err) {
      props.notify(err instanceof Error ? err.message : 'Could not start the package revision.', true);
    }
  }
  async function accept() {
    if (!run) return;
    try {
      const saved = await client.saveForgeRevise(workspaceId, run.runId);
      setSavedVersion(saved.artifact.version);
      props.notify(
        saved.alreadySaved
          ? 'This revision was already saved.'
          : `Revision saved as version ${saved.artifact.version} of artifact ${saved.artifact.artifactId}.`,
        false,
      );
      props.onSaved();
    } catch (err) {
      props.notify(err instanceof Error ? err.message : 'Could not save the revision.', true);
    }
  }
  function reject() {
    // Rejection saves nothing: no save call is made, so the
    // parent package version is untouched by construction.
    setRun(null);
    setRejected(true);
  }
  const diff = run?.revisionDiff;
  return (
    <div>
      <h4>Revise this source package</h4>
      <p className="help">
        Revising artifact {pkg.artifactId} version {pkg.version}, from specification artifact{' '}
        {doc.specArtifactId} version {doc.specVersion}. The original version stays unchanged
        until you explicitly accept a revision, which saves a new version of this artifact.
        A revision never switches specifications and never creates specification approval:
        the pinned specification version must already be approved, or the revision is refused
        before any provider traffic. The proposed files are inert text — ANANSI does not run,
        preview, build or test them.
      </p>
      <form onSubmit={e => void start(e)}>
        <label htmlFor="forge-revision-instructions">Revision instructions — what should change?</label>
        <textarea
          id="forge-revision-instructions"
          rows={3}
          value={instructions}
          onChange={e => setInstructions(e.target.value)}
          required
        />
        <label>
          <input type="checkbox" checked={lock} onChange={e => setLock(e.target.checked)} /> Zero-Cost Lock (never spend)
        </label>
        <button type="submit" disabled={running}>Request revision</button>
        {running && (
          <button
            type="button"
            className="secondary"
            disabled={run?.status === 'cancelling'}
            onClick={() => void (async () => {
              const cancelled = await controller().cancel();
              if (cancelled) setRun(cancelled);
            })()}
          >
            Cancel
          </button>
        )}
      </form>
      {run?.status === 'running' && <p role="status">Revising… {run.partialChars} characters received.</p>}
      {run?.status === 'cancelling' && <p role="status">Cancelling… waiting for the provider stream to stop.</p>}
      {run?.status === 'cancelled' && <p role="status">Revision cancelled. Nothing was saved.</p>}
      {run && (run.status === 'failed' || run.status === 'invalid') && (
        <p role="alert">Revision {run.status}: {run.error ?? 'the provider did not produce a usable revised package.'} Nothing was saved; you can try again.</p>
      )}
      {rejected && <p role="status">Revision rejected. Nothing was saved; the original package version is unchanged.</p>}
      {disclosure && <DisclosurePanel disclosure={disclosure} />}
      {run?.sourcePackage && diff && (
        <div>
          <h5>Proposed changes (against version {pkg.version})</h5>
          {diff.added.length > 0 && (
            <p>Added: <span aria-label="Revision added files">{diff.added.join(', ')}</span></p>
          )}
          {diff.removed.length > 0 && (
            <p>Removed: <span aria-label="Revision removed files">{diff.removed.join(', ')}</span></p>
          )}
          {diff.changed.length > 0 && (
            <p>Changed: <span aria-label="Revision changed files">{diff.changed.join(', ')}</span></p>
          )}
          {diff.added.length === 0 && diff.removed.length === 0 && diff.changed.length === 0 && (
            <p>No file changes proposed.</p>
          )}
          <SourcePackageView doc={run.sourcePackage} />
          {savedVersion > 0 ? (
            <p role="status">Revision saved as version {savedVersion}. Open the new version from the Artifacts list to inspect and export it.</p>
          ) : (
            <div>
              <button type="button" onClick={() => void accept()}>
                Accept revision — save as new version
              </button>{' '}
              <button type="button" className="secondary" onClick={() => reject()}>
                Reject revision — save nothing
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
