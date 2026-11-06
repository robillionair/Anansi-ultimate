# LARP SwarmTank arena

Baseline `3bf51af85`. Acceptance requires an eighth LARP scene, keyboard and dock navigation, project selection, the existing local SwarmTank simulation and report workflow, scenario controls and persona inspection. The arena must use LARP lighting and effects, support orbit, pan and zoom, and preserve reduced motion and the existing table fallback. Decisions shown in the arena must come from the existing simulator. Verify the integrated browser workflow and the existing SwarmTank and LARP regressions.

## Browser acceptance

Open a workspace and choose SwarmTank from the LARP dock or press 8. Choose a project or create the first project from an empty workspace. Enter a product description and optional pricing, segment, and feature details. Generate personas, then evaluate decisions to save the run. The arena supports drag to orbit, scroll to zoom, right-drag to pan, and Home or Reset arena camera to restore the view. Select a persona in the arena or report table to read its details. Change the scenario price to see the outcome counts and clustering update. Reopen the saved report from the same project after leaving the scene.

Windows browser evidence covers generating and saving a run, orbit and zoom, camera reset, table selection, reduced motion, a price change updating the outcomes, and reopening the saved report with zero provider calls. The existing LARP navigation and SwarmTank arena interaction and reduced-motion tests passed. The existing no-WebGL test passed on rerun after restoring the Projects fallback wording. UI and contracts passed 320/320.

## Cinematic presentation pass

Acceptance: a distinct silhouette and legible data at normal desktop scale, with the same camera, persona inspection, scenario updates, reduced-motion and saved-run behavior. All added scene geometry is decorative and must leave simulation data and hit testing unchanged.

## Matrix presentation

Acceptance: show a distinct interactive dot for each simulated persona, render decorative cables that follow the dot positions during scenario changes, and display readable cards sourced from the active run. Preserve pick targets, camera control, reduced motion, table fallback, and saved runs.

The matrix view opens by default on an evaluated or saved run. Select a card to highlight its cables and inspect the persona, or use the page controls to reach the remaining personas. Toggle Persona matrix off for the unobstructed 3D network. Both views use the current scenario decisions. Cables are decorative and do not represent inferred links between personas.
