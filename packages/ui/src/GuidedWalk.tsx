// M12 Slice 3: guided lifecycle walk panel.
//
// An ordered, plain-language checklist over the pipeline state (see
// `guidedWalk.ts`), mounted in the project view right after the
// PipelineNavigator. It performs no server calls of its own: it derives
// from the project's already-loaded artifact types, so it refreshes
// whenever the project view reloads artifacts after a panel save.
// Navigation targets are the project view's anchored sections, except
// Refine, which lives in the Ask ANANSI view and needs the
// `onNavigateToAsk` callback, and Launch, which opens the Launch Studio
// section via the `onSelectLaunch` callback (same defensive shape as
// `PipelineNavigator`: callback buttons render only when provided).
import React from 'react';
import { deriveWalk, type WalkStep } from './guidedWalk.ts';

export function GuidedWalk({
  artifactTypes,
  onNavigateToAsk,
  onSelectLaunch,
}: {
  artifactTypes: string[];
  onNavigateToAsk?: () => void;
  onSelectLaunch?: () => void;
}) {
  const { steps, continueStep, complete } = deriveWalk(artifactTypes);
  const continueTarget = continueStep
    ? steps.find((s) => s.id === continueStep)!
    : null;

  function stepButton(step: WalkStep) {
    const label = `Go to ${step.label}`;
    if (step.action === 'ask') {
      if (!onNavigateToAsk) return null;
      return (
        <button type="button" onClick={onNavigateToAsk}>
          {label}
        </button>
      );
    }
    if (step.action === 'launch') {
      if (!onSelectLaunch) return null;
      return (
        <button type="button" onClick={onSelectLaunch}>
          {label}
        </button>
      );
    }
    return <a href={step.anchor ?? '#'}>{label}</a>;
  }

  function continueControl() {
    if (!continueTarget) return null;
    const label = 'Continue walk';
    if (continueTarget.action === 'ask') {
      if (!onNavigateToAsk) return null;
      return (
        <button type="button" onClick={onNavigateToAsk}>
          {label}
        </button>
      );
    }
    if (continueTarget.action === 'launch') {
      if (!onSelectLaunch) return null;
      return (
        <button type="button" onClick={onSelectLaunch}>
          {label}
        </button>
      );
    }
    return <a href={continueTarget.anchor ?? '#'}>{label}</a>;
  }

  return (
    <section aria-labelledby="project-walk-heading">
      <h3 id="project-walk-heading">Guided Walk</h3>
      <p>
        Follow the creation pipeline in order, from opportunity to launch
        materials. Each step jumps to the tool that performs it.
      </p>
      {complete ? (
        <p>
          Walk complete — all five completable steps are done. Refine stays
          available as an ongoing practice; it is not a deliverable and
          never reads Complete.
        </p>
      ) : (
        continueControl()
      )}
      <ol>
        {steps.map((step) => (
          <li key={step.id}>
            <strong>{step.label}</strong>
            <span> · {step.status}</span>
            {step.missing && <span> · {step.missing}</span>}
            <p>{step.instruction}</p>
            {stepButton(step)}
          </li>
        ))}
      </ol>
    </section>
  );
}
