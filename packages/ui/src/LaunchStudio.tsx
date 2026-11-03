// M12 Slice 2: Launch Studio.
//
// Mounted in the project view when the Launch stage is selected. It reads
// only the project artifacts the user checks, generates deterministic
// marketing copy from their parsed bodies, and downloads a deterministic
// ZIP bundle. The deterministic path sends nothing anywhere; the provider
// path sends only the previewed prompt, and only after explicit approval.
import React, { useState } from 'react';
import type { ArtifactRecord } from '@anansi/contracts';
import { parseTransferConceptBody } from './Transfer.tsx';
import { parseProductSpecBody } from './Forge.tsx';
import {
  deriveConsolidationProposals,
  parseSomniaEvaluationDoc,
} from './Somnia.tsx';
import { parseSwarmtankRunBody } from './Swarmtank.tsx';
import {
  buildLaunchProviderPrompt,
  buildSomniaNotes,
  buildSwarmtankSummary,
  deriveSimReport,
  estimateTokens,
  generateLaunchCopy,
  launchGate,
  runLaunchProviderDraft,
  type LaunchChannels,
  type LaunchEvidence,
} from './launchCopy.ts';
import {
  buildLaunchBundle,
  buildLaunchManifest,
  type BundleFile,
} from './launchBundle.ts';

const LAUNCH_TYPES = [
  'concept',
  'specification',
  'source-package',
  'deliverable',
  'simulation',
  'scenario',
  'somnia-evaluation',
];

function ChannelCard({ title, text }: { title: string; text: string }) {
  const [tab, setTab] = useState<'preview' | 'source'>('preview');
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }
  return (
    <article aria-label={`${title} preview`}>
      <h4>{title}</h4>
      <div>
        <button type="button" onClick={() => setTab('preview')}>
          Preview
        </button>
        <button type="button" onClick={() => setTab('source')}>
          Source
        </button>
        <button type="button" onClick={copy}>
          Copy to clipboard
        </button>
        {copied && <span> Copied.</span>}
      </div>
      {tab === 'preview' ? (
        <div>
          {text.split('\n\n').map((paragraph, i) => (
            <p key={i} style={{ whiteSpace: 'pre-wrap' }}>
              {paragraph}
            </p>
          ))}
        </div>
      ) : (
        <pre>{text}</pre>
      )}
    </article>
  );
}

export function LaunchStudio({
  artifacts,
  readArtifact,
  dispatch,
  notify,
}: {
  artifacts: ArtifactRecord[];
  readArtifact: (artifactId: string, version: number) => Promise<string>;
  dispatch?: (prompt: string) => Promise<string>;
  notify?: (message: string, isError: boolean) => void;
}) {
  const relevant = artifacts.filter(a => LAUNCH_TYPES.includes(a.type));
  const [selected, setSelected] = useState<string[]>(
    relevant.map(a => a.artifactId),
  );
  const [copy, setCopy] = useState<LaunchChannels | null>(null);
  const [missing, setMissing] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [evidence, setEvidence] = useState<LaunchEvidence | null>(null);
  const [bodies, setBodies] = useState<Map<string, string>>(new Map());
  const [chosen, setChosen] = useState<ArtifactRecord[]>([]);
  const [providerPrompt, setProviderPrompt] = useState<string | null>(null);
  const [providerDraft, setProviderDraft] = useState<string | null>(null);
  const [providerBusy, setProviderBusy] = useState(false);
  const [includeDraft, setIncludeDraft] = useState(false);

  function toggle(artifactId: string) {
    setSelected(current =>
      current.includes(artifactId)
        ? current.filter(id => id !== artifactId)
        : [...current, artifactId],
    );
  }

  async function generate() {
    setError(null);
    setMissing([]);
    setGenerating(true);
    try {
      const chosenRecords = relevant.filter(a =>
        selected.includes(a.artifactId),
      );
      const readBodies = new Map<string, string>();
      let conceptDoc = null;
      let specDoc = null;
      let runDoc = null;
      let evalDoc = null;
      for (const record of chosenRecords) {
        const body = await readArtifact(record.artifactId, record.version);
        readBodies.set(record.artifactId, body);
        if (record.type === 'concept') {
          conceptDoc = parseTransferConceptBody(body);
        } else if (record.type === 'specification') {
          specDoc = parseProductSpecBody(body);
        } else if (record.type === 'simulation') {
          runDoc = parseSwarmtankRunBody(body);
        } else if (record.type === 'somnia-evaluation') {
          evalDoc = parseSomniaEvaluationDoc(body);
        }
      }
      const missingList = launchGate(!!conceptDoc, !!runDoc, !!evalDoc);
      if (missingList.length > 0) {
        setMissing(missingList);
        setCopy(null);
        setEvidence(null);
        setChosen([]);
        setBodies(new Map());
        return;
      }
      const built: LaunchEvidence = {
        conceptName: conceptDoc!.selectedCombination.name,
        conceptDescription: conceptDoc!.selectedCombination.description,
        productizationBrief: conceptDoc!.productizationBrief,
        specTitle: specDoc?.spec.title,
        specSummary: specDoc?.spec.summary,
        specPurpose: specDoc?.spec.purpose,
        specIntendedUsers: specDoc?.spec.intendedUsers,
        specFeatures: specDoc?.spec.features,
        specPages: specDoc?.spec.pages,
        specDataAndState: specDoc?.spec.dataAndState ?? undefined,
        simInput: runDoc?.input,
        simReport: runDoc
          ? deriveSimReport(runDoc.decisions, runDoc.personas)
          : undefined,
        somnia: evalDoc
          ? {
              scenarios: evalDoc.input.scenarios.map(s => ({
                id: s.id,
                instruction: s.instruction,
              })),
              results: evalDoc.results,
              scenariosTotal: evalDoc.metrics.scenariosTotal,
              completed: evalDoc.metrics.completed,
              refused: evalDoc.metrics.refused,
              aborted: evalDoc.metrics.aborted,
              simulatedToolCalls: evalDoc.metrics.simulatedToolCalls,
              violationsByCategory: evalDoc.metrics.violationsByCategory,
              proposals: deriveConsolidationProposals(evalDoc),
            }
          : undefined,
      };
      setEvidence(built);
      setBodies(readBodies);
      setChosen(chosenRecords);
      setCopy(
        generateLaunchCopy(
          built,
          chosenRecords.map(r => ({
            type: r.type,
            title: r.title,
            version: r.version,
          })),
        ),
      );
      setProviderPrompt(null);
      setProviderDraft(null);
      setIncludeDraft(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Generation failed.');
    } finally {
      setGenerating(false);
    }
  }

  function download() {
    if (!copy) return;
    const encoder = new TextEncoder();
    const files: BundleFile[] = [
      { path: 'marketing/show-hn.md', content: encoder.encode(copy.showHn) },
      {
        path: 'marketing/product-hunt.md',
        content: encoder.encode(copy.productHunt),
      },
      {
        path: 'marketing/x-thread.md',
        content: encoder.encode(copy.xThread.join('\n\n')),
      },
      {
        path: 'marketing/release-notes.md',
        content: encoder.encode(copy.releaseNotes),
      },
    ];
    if (evidence) {
      const summary = buildSwarmtankSummary(evidence);
      if (summary) {
        files.push({
          path: 'swarmtank-summary.md',
          content: encoder.encode(summary),
        });
      }
      const notes = buildSomniaNotes(evidence);
      if (notes) {
        files.push({
          path: 'somnia-pressure-test-notes.md',
          content: encoder.encode(notes),
        });
      }
    }
    for (const record of chosen) {
      files.push({
        path: `artifacts/${record.type}-${record.artifactId}-v${record.version}.json`,
        content: encoder.encode(bodies.get(record.artifactId) ?? ''),
      });
    }
    const draftIncluded = includeDraft && providerDraft !== null;
    if (draftIncluded) {
      files.push({
        path: 'marketing/provider-draft.md',
        content: encoder.encode(providerDraft!),
      });
    }
    files.push({
      path: 'LAUNCH-MANIFEST.json',
      content: encoder.encode(
        buildLaunchManifest(
          chosen.map(r => ({
            artifactId: r.artifactId,
            type: r.type,
            version: r.version,
            contentHashPrefix: r.contentHash.slice(0, 16),
          })),
          { providerDraft: draftIncluded, providerLabel: null },
        ),
      ),
    });
    const zip = buildLaunchBundle(files);
    const url = URL.createObjectURL(
      new Blob([zip.buffer as ArrayBuffer], { type: 'application/zip' }),
    );
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = 'anansi-launch-bundle.zip';
    anchor.click();
    URL.revokeObjectURL(url);
    notify?.(
      `Downloaded anansi-launch-bundle.zip (${files.length} files).`,
      false,
    );
  }

  function previewProviderPrompt() {
    if (!evidence) return;
    setProviderPrompt(buildLaunchProviderPrompt(evidence));
    setProviderDraft(null);
  }

  async function approveProvider() {
    if (!providerPrompt || !dispatch) return;
    setProviderBusy(true);
    try {
      const draft = await runLaunchProviderDraft(dispatch, providerPrompt);
      setProviderDraft(draft);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Provider draft failed.');
    } finally {
      setProviderBusy(false);
    }
  }

  return (
    <div>
      <p>
        Launch Studio generates marketing copy from the project artifacts you
        select, using deterministic rules only. No provider call happens
        unless you explicitly approve one below. The bundle download is local
        only; nothing is published, deployed, or registered.
      </p>
      {relevant.length === 0 && (
        <p>
          No launch evidence artifacts in this project yet. Register a Forge
          concept and a SwarmTank simulation or SOMNIA evaluation first.
        </p>
      )}
      {relevant.map(record => (
        <label key={record.artifactId}>
          <input
            type="checkbox"
            checked={selected.includes(record.artifactId)}
            onChange={() => toggle(record.artifactId)}
          />
          {`Include ${record.type}: ${record.title} (version ${record.version})`}
        </label>
      ))}
      <div>
        <button
          type="button"
          onClick={generate}
          disabled={generating || relevant.length === 0}
        >
          Generate launch copy
        </button>
      </div>
      {missing.length > 0 && (
        <p>
          Cannot generate yet — missing: {missing.join('; ')}. Select the
          artifacts above, or create them in their tools first.
        </p>
      )}
      {error && <p>{error}</p>}
      {copy && (
        <div>
          <ChannelCard title="Show HN" text={copy.showHn} />
          <ChannelCard title="Product Hunt" text={copy.productHunt} />
          <ChannelCard
            title="X release thread"
            text={copy.xThread.join('\n\n')}
          />
          <ChannelCard title="Release notes" text={copy.releaseNotes} />
          <div>
            <button type="button" onClick={download}>
              Download Launch Bundle (.zip)
            </button>
          </div>
          <section aria-label="Provider draft">
            <h4>Generate a provider draft (optional)</h4>
            <p>
              The deterministic copy above is the default. A provider draft is
              a separate preview that never replaces it. Nothing is sent until
              you approve the exact prompt below.
            </p>
            <button
              type="button"
              onClick={previewProviderPrompt}
              disabled={!evidence}
            >
              Preview provider prompt
            </button>
            {providerPrompt !== null && (
              <div>
                <p>
                  Estimated input tokens: {estimateTokens(providerPrompt)}{' '}
                  (estimated). Check this against your configured provider
                  budget in Providers before approving; the Ask dispatch path
                  enforces that budget server-side.
                </p>
                <pre>{providerPrompt}</pre>
                <button
                  type="button"
                  onClick={approveProvider}
                  disabled={providerBusy || !dispatch}
                >
                  Approve and send to Ask ANANSI
                </button>
                {!dispatch && (
                  <span> Provider dispatch is unavailable here.</span>
                )}
              </div>
            )}
            {providerDraft !== null && (
              <div>
                <ChannelCard title="Provider draft" text={providerDraft} />
                <label>
                  <input
                    type="checkbox"
                    checked={includeDraft}
                    onChange={e => setIncludeDraft(e.target.checked)}
                  />
                  Include provider draft in the bundle
                </label>
              </div>
            )}
          </section>
        </div>
      )}
    </div>
  );
}
