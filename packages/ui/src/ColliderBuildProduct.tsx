// "Build This Product": the one-click approved-outputs
// path into Product Forge. It calls the EXISTING
// `collider-transfer` op — no new backend op, no new
// Forge entity. The approved brief is the record's own
// concept brief (the same text the derived CONCEPT.md
// carries in its Concept brief section); it always
// fits the transfer op's 4000-character brief bound,
// while the full CONCEPT.md text would not. The op
// embeds the derived ARCHITECTURE.md as the concept's
// initial specification and pins the experiment id,
// version, and content hash. Re-running Build for the
// same combination returns the same concept with
// `alreadyTransferred` — never a duplicate. Transfer
// approves NO specification and starts NO provider
// run: the ForgeSpecPanel on the concept page remains
// the separate explicit spec action.
import { useState } from 'react';
import type { Client, ColliderExperimentDoc } from '@anansi/contracts';

export function ColliderBuildProduct({
  client,
  workspaceId,
  projectId,
  doc,
  experimentArtifactId,
  experimentVersion,
  notify,
  onTransferred,
  busy,
}: {
  client: Client;
  workspaceId: string;
  projectId: string;
  doc: ColliderExperimentDoc;
  experimentArtifactId: string;
  experimentVersion: number;
  notify: (message: string, isError: boolean) => void;
  onTransferred: (conceptId: string) => void;
  busy: boolean;
}) {
  const [combinationIndex, setCombinationIndex] = useState(0);
  const [running, setRunning] = useState(false);
  async function build() {
    setRunning(true);
    try {
      const transferred = await client.transferProjectCollider(workspaceId, {
        projectId,
        experimentArtifactId,
        experimentVersion,
        combinationIndex,
        approvedBrief: doc.result.productization.conceptBrief,
      });
      if (transferred.alreadyTransferred) {
        notify('Already built: opened the existing Forge concept for this combination.', false);
      } else {
        notify('Built: Forge concept created from the approved outputs.', false);
      }
      onTransferred(transferred.artifact.artifactId);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Build failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }
  return (
    <section aria-label="Build this product">
      <h3>Build This Product</h3>
      <p className="help">
        Creates a Product Forge concept from the approved outputs of this saved
        experiment: the concept brief, the derived ARCHITECTURE.md as the initial
        technical specification, and the pinned experiment id, version, and
        content hash. Building twice with the same combination reuses the same
        concept. This does not start specification generation — that stays a
        separate explicit action on the concept page.
      </p>
      <label>
        Combination to build{' '}
        <select
          aria-label="Combination to build"
          value={combinationIndex}
          onChange={e => setCombinationIndex(Number(e.target.value))}
        >
          {doc.result.combinations.map((combination, i) => (
            <option key={i} value={i}>
              {combination.name}
            </option>
          ))}
        </select>
      </label>{' '}
      <button type="button" disabled={busy || running} onClick={() => void build()}>
        {running ? 'Building…' : 'Build This Product'}
      </button>
    </section>
  );
}
