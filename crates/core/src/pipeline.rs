//! M12 Slice 1: unified product creation pipeline stage engine.
//!
//! Pure functions over artifact type strings. The UI derives the input
//! from the already-loaded `listProjectArtifacts` response, so inspecting
//! stages performs zero server mutations and needs no model. Unknown
//! artifact types are ignored, never errors: this engine is a view over
//! the registry, not a validator of it.
//!
//! The TypeScript mirror in `packages/ui/src/lifecycle.ts` implements the
//! same stages, gates, evidence sets, and status rule. The two must stay
//! in lockstep; the unit tests on both sides pin the same cases.
//!
//! Note on placement: `crates/core/src/lifecycle.rs` already owns the
//! memory-state machine (`MemoryState`), so the pipeline engine lives
//! here instead of colliding with it.

/// The seven Slice 1 pipeline stages, in pipeline order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Discover,
    Invent,
    Build,
    Simulate,
    PressureTest,
    Refine,
    Launch,
}

impl PipelineStage {
    /// All stages, in pipeline order.
    pub fn all() -> [PipelineStage; 7] {
        [
            PipelineStage::Discover,
            PipelineStage::Invent,
            PipelineStage::Build,
            PipelineStage::Simulate,
            PipelineStage::PressureTest,
            PipelineStage::Refine,
            PipelineStage::Launch,
        ]
    }

    /// Stable machine id, matching the assignment's stage ids.
    pub fn id(self) -> &'static str {
        match self {
            PipelineStage::Discover => "discover",
            PipelineStage::Invent => "invent",
            PipelineStage::Build => "build",
            PipelineStage::Simulate => "simulate",
            PipelineStage::PressureTest => "pressure_test",
            PipelineStage::Refine => "refine",
            PipelineStage::Launch => "launch",
        }
    }

    /// Human label shown in the navigator.
    pub fn label(self) -> &'static str {
        match self {
            PipelineStage::Discover => "Discover",
            PipelineStage::Invent => "Invent",
            PipelineStage::Build => "Build",
            PipelineStage::Simulate => "Simulate",
            PipelineStage::PressureTest => "Pressure test",
            PipelineStage::Refine => "Refine",
            PipelineStage::Launch => "Launch",
        }
    }

    /// Artifact types whose presence completes this stage. `Refine`
    /// has none: GHOST-PROMPT sessions are in-memory only and are
    /// never registered, so refine never auto-completes.
    pub fn evidence_types(self) -> &'static [&'static str] {
        match self {
            PipelineStage::Discover => &["idea"],
            PipelineStage::Invent => &["concept"],
            PipelineStage::Build => &[
                "specification",
                "source-package",
                "deliverable",
                "source-review",
            ],
            PipelineStage::Simulate => &["simulation", "scenario"],
            PipelineStage::PressureTest => &["somnia-evaluation"],
            PipelineStage::Refine => &[],
            PipelineStage::Launch => &["launch-material"],
        }
    }

    /// Whether this stage can ever be the recommended (Current) stage.
    /// Refine cannot: with no evidence it can never be "in progress"
    /// in an artifact-derived model, and it must never block launch.
    pub fn is_advanceable(self) -> bool {
        !matches!(self, PipelineStage::Refine)
    }
}

/// Navigator status for one stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStatus {
    Complete,
    Current,
    Available,
    Awaiting,
}

impl PipelineStatus {
    /// Label shown in the navigator.
    pub fn label(self) -> &'static str {
        match self {
            PipelineStatus::Complete => "Complete",
            PipelineStatus::Current => "Current / In Progress",
            PipelineStatus::Available => "Available",
            PipelineStatus::Awaiting => "Awaiting Preceding Stage",
        }
    }
}

/// One stage's derived state: status, per-type evidence counts (only
/// types actually present, in evidence order), and the requirement
/// sentence when the stage is awaiting its gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineStageState {
    pub stage: PipelineStage,
    pub status: PipelineStatus,
    pub evidence_counts: Vec<(&'static str, usize)>,
    pub missing: Option<&'static str>,
}

/// The whole pipeline derivation: one state per stage plus the
/// recommended stage, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineDerivation {
    pub stages: Vec<PipelineStageState>,
    pub current: Option<PipelineStage>,
}

/// Whether `stage`'s gate is met for the given artifact types.
fn gate_met(stage: PipelineStage, has: &dyn Fn(&str) -> bool) -> bool {
    match stage {
        // Always open: §62 starts from a user idea, and GHOST-PROMPT
        // is an editor-scoped tool usable at any time.
        PipelineStage::Discover | PipelineStage::Invent | PipelineStage::Refine => true,
        // Forge, SwarmTank, and the SOMNIA QA bridge all work from a
        // Forge concept.
        PipelineStage::Build | PipelineStage::Simulate | PipelineStage::PressureTest => {
            has("concept")
        }
        // A completed simulation or evaluation artifact.
        PipelineStage::Launch => {
            has("simulation") || has("scenario") || has("somnia-evaluation") || has("evaluation")
        }
    }
}

/// The requirement sentence shown while a stage awaits its gate.
fn missing_sentence(stage: PipelineStage) -> Option<&'static str> {
    match stage {
        PipelineStage::Build | PipelineStage::Simulate | PipelineStage::PressureTest => {
            Some("Needs at least one Forge concept artifact.")
        }
        PipelineStage::Launch => Some("Needs a completed simulation or evaluation artifact."),
        _ => None,
    }
}

/// Derive the pipeline state from artifact type strings (duplicates
/// count: two concepts are two pieces of evidence).
pub fn derive_pipeline(artifact_types: &[&str]) -> PipelineDerivation {
    let has = |t: &str| artifact_types.contains(&t);
    let stages = PipelineStage::all();
    let has_evidence: Vec<bool> = stages
        .iter()
        .map(|s| s.evidence_types().iter().any(|t| has(t)))
        .collect();
    // Highest stage index with completing evidence, or None.
    let furthest = has_evidence.iter().rposition(|e| *e);
    // Current: first advanceable stage after `furthest` whose gate is
    // met. Scanning after `furthest` (never from the start) keeps the
    // recommendation moving forward past work already done.
    let current = stages.iter().enumerate().find_map(|(i, s)| {
        let after_furthest = furthest.is_none_or(|f| i > f);
        if after_furthest && s.is_advanceable() && gate_met(*s, &has) {
            Some(*s)
        } else {
            None
        }
    });
    let states = stages
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let status = if has_evidence[i] {
                // Evidence always beats a missing gate: a directly
                // registered simulation without a concept still shows
                // Simulate Complete.
                PipelineStatus::Complete
            } else if Some(*s) == current {
                PipelineStatus::Current
            } else if gate_met(*s, &has) {
                PipelineStatus::Available
            } else {
                PipelineStatus::Awaiting
            };
            let evidence_counts = s
                .evidence_types()
                .iter()
                .map(|t| (*t, artifact_types.iter().filter(|a| **a == *t).count()))
                .filter(|(_, c)| *c > 0)
                .collect();
            PipelineStageState {
                stage: *s,
                status,
                evidence_counts,
                missing: if status == PipelineStatus::Awaiting {
                    missing_sentence(*s)
                } else {
                    None
                },
            }
        })
        .collect();
    PipelineDerivation {
        stages: states,
        current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn statuses(types: &[&str]) -> Vec<PipelineStatus> {
        derive_pipeline(types)
            .stages
            .iter()
            .map(|s| s.status)
            .collect()
    }

    #[test]
    fn fresh_project_starts_at_discover() {
        let d = derive_pipeline(&[]);
        assert_eq!(d.current, Some(PipelineStage::Discover));
        assert_eq!(
            statuses(&[]),
            vec![
                PipelineStatus::Current,
                PipelineStatus::Available,
                PipelineStatus::Awaiting,
                PipelineStatus::Awaiting,
                PipelineStatus::Awaiting,
                PipelineStatus::Available,
                PipelineStatus::Awaiting,
            ]
        );
    }

    #[test]
    fn concept_only_advances_to_build() {
        assert_eq!(
            statuses(&["concept"]),
            vec![
                PipelineStatus::Available,
                PipelineStatus::Complete,
                PipelineStatus::Current,
                PipelineStatus::Available,
                PipelineStatus::Available,
                PipelineStatus::Available,
                PipelineStatus::Awaiting,
            ]
        );
    }

    #[test]
    fn concept_plus_simulation_advances_to_pressure_test() {
        let d = derive_pipeline(&["concept", "simulation"]);
        assert_eq!(d.current, Some(PipelineStage::PressureTest));
        assert_eq!(d.stages[1].status, PipelineStatus::Complete);
        assert_eq!(d.stages[3].status, PipelineStatus::Complete);
        assert_eq!(d.stages[6].status, PipelineStatus::Available);
        assert_eq!(d.stages[6].missing, None);
    }

    #[test]
    fn pressure_test_complete_makes_launch_current_and_refine_available() {
        let d = derive_pipeline(&["concept", "simulation", "somnia-evaluation"]);
        assert_eq!(d.current, Some(PipelineStage::Launch));
        assert_eq!(d.stages[4].status, PipelineStatus::Complete);
        // Refine is never Current and never a blocker.
        assert_eq!(d.stages[5].status, PipelineStatus::Available);
        assert_eq!(d.stages[6].status, PipelineStatus::Current);
    }

    #[test]
    fn full_pipeline_has_no_current_stage() {
        let d = derive_pipeline(&[
            "idea",
            "concept",
            "specification",
            "simulation",
            "somnia-evaluation",
            "launch-material",
        ]);
        assert_eq!(d.current, None);
        assert_eq!(d.stages[5].status, PipelineStatus::Available);
        assert!(
            d.stages
                .iter()
                .filter(|s| s.stage != PipelineStage::Refine)
                .all(|s| s.status == PipelineStatus::Complete)
        );
    }

    #[test]
    fn evidence_beats_a_missing_gate_and_unknown_types_are_ignored() {
        // A simulation with no concept still completes Simulate.
        let d = derive_pipeline(&["simulation", "mystery-type"]);
        assert_eq!(d.stages[3].status, PipelineStatus::Complete);
        assert_eq!(d.stages[1].status, PipelineStatus::Available);
        assert_eq!(d.stages[2].status, PipelineStatus::Awaiting);
        assert_eq!(
            d.stages[2].missing,
            Some("Needs at least one Forge concept artifact.")
        );
        // Launch gate is met by the simulation even without a concept.
        assert_eq!(d.current, Some(PipelineStage::Launch));
    }

    #[test]
    fn evidence_counts_count_duplicates_in_order() {
        let d = derive_pipeline(&["concept", "concept", "simulation"]);
        assert_eq!(d.stages[1].evidence_counts, vec![("concept", 2)]);
        assert_eq!(d.stages[3].evidence_counts, vec![("simulation", 1)]);
        assert!(d.stages[0].evidence_counts.is_empty());
    }
}
