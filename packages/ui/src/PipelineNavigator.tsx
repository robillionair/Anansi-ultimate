// M12 Slice 1: unified product creation pipeline navigator.
//
// A visual stepper over the artifact-derived pipeline state (see
// `lifecycle.ts`). It performs no server calls of its own: it derives
// from the project's already-loaded artifact types, so it refreshes
// whenever the project view reloads artifacts after a panel save.
// Navigation targets are the project view's anchored sections, except
// Refine, which lives in the Ask ANANSI view and needs the
// `onNavigateToAsk` callback, and Launch, which opens the Launch Studio
// section via the `onSelectLaunch` callback (M12 Slice 2).
import React from 'react';
import { derivePipeline, type PipelineStageId } from './lifecycle.ts';

const TOOL: Record<PipelineStageId, { name: string; anchor: string | null }> = {
  discover: { name: 'Opportunity discovery', anchor: '#project-opportunities-heading' },
  invent: { name: 'LEXI-COLLIDER', anchor: '#project-collider-heading' },
  build: { name: 'Artifacts (Product Forge)', anchor: '#project-artifacts-heading' },
  simulate: { name: 'SwarmTank', anchor: '#project-swarmtank-heading' },
  pressure_test: { name: 'SOMNIA', anchor: '#project-somnia-heading' },
  refine: { name: 'Ask ANANSI (GHOST-PROMPT)', anchor: null },
  launch: { name: 'Launch Studio', anchor: null },
};

export function PipelineNavigator({
  artifactTypes,
  onNavigateToAsk,
  onSelectLaunch,
}: {
  artifactTypes: string[];
  onNavigateToAsk?: () => void;
  onSelectLaunch?: () => void;
}) {
  const { stages, current } = derivePipeline(artifactTypes);
  const allCompletableComplete = stages
    .filter((s) => s.id !== 'refine')
    .every((s) => s.status === 'Complete');
  return (
    <nav aria-label="Product Creation Pipeline">
      <h3>Product Creation Pipeline</h3>
      {current === null && allCompletableComplete && (
        <p>Pipeline complete: launch materials are registered.</p>
      )}
      {current === null && !allCompletableComplete && (
        <p>No recommended next stage — jump to any available tool.</p>
      )}
      <ol>
        {stages.map((stage) => {
          const tool = TOOL[stage.id];
          const isCurrent = current === stage.id;
          const counts = stage.evidenceCounts
            .map((c) => `${c.count} ${c.type}`)
            .join(' · ');
          return (
            <li key={stage.id} aria-current={isCurrent ? 'step' : undefined}>
              <strong>{stage.label}</strong>
              <span> · {stage.status}</span>
              {counts && <span> · {counts}</span>}
              {stage.missing && <span> · {stage.missing}</span>}
              {' '}
              {stage.id === 'launch' ? (
                <button
                  type="button"
                  disabled={!onSelectLaunch}
                  onClick={onSelectLaunch}
                >
                  {stage.status === 'Awaiting Preceding Stage'
                    ? 'Jump to Launch Studio'
                    : 'Open Launch Studio'}
                </button>
              ) : stage.id === 'refine' ? (
                <button
                  type="button"
                  disabled={!onNavigateToAsk}
                  onClick={onNavigateToAsk}
                >
                  Open {tool.name}
                </button>
              ) : tool.anchor ? (
                <a href={tool.anchor}>
                  {stage.status === 'Awaiting Preceding Stage'
                    ? `Jump to ${tool.name}`
                    : `Open ${tool.name}`}
                </a>
              ) : null}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
