# ANANSI Ultimate 2.0 — Master plan addendum

**Status** · Planned. Finish and release ANANSI Ultimate 1.0 before starting implementation of this package. Deliver 2.0 in testable slices under the same release plan.

## Nanzi AI teammates

One teammate is a **Nanzi**. Multiple teammates are **Nanzis**.

Nanzis are persistent AI teammates that can be assigned a role and tasks inside an ANANSI project. A founder might create a Research Nanzi that reads approved notes, lists conflicting evidence, and drafts a brief. The owner can reopen the same teammate later, inspect its prior runs, and decide which outputs become project artifacts.

The Grok Bot-style reference is Rakazo. Its persistent teammates, model choice, routines, shared conversations, and isolated computers are product and architecture references for later design. [Rakazo repository](https://github.com/elie222/rakazo)

### Initial Nanzi workflow

1. Create a Nanzi in an existing project. Give it a name, role, instructions, and a configured model. Keep the profile and run history available after restart.
2. Review which project records the Nanzi can use. Start a task explicitly and show the active model, selected context, run status, and disclosure through ANANSI's existing provider controls.
3. Review a cited result and explicitly save it as a versioned project artifact. Support cancellation and show what completed before the stop. Let the owner revoke context access and delete the teammate's saved state.

### Nanzi follow-up slices

- **Tools and routines.** Add scoped, read-only MCP tools first. Add explicit approval for writes and opt-in scheduling with visible run history, limits, and a stop control.
- **Isolated computer.** Add browser, file, and terminal actions only after sandbox and permission acceptance. Give the owner a way to inspect activity and take control.
- **Collaboration.** Allow multiple Nanzis to work in a shared project conversation. Show which teammate produced each result, and bound delegation by permissions and budget.

## OpenJEV-inspired decision layer

Evaluate a fast, structured decision layer for choosing the next ANANSI workflow, classifying incoming files, suggesting groups, and deciding when to ask the user for clarification. OpenJEV is a reference for model-backed yes/no decisions, constrained choices, and scoring. Start with a benchmark against simple rules and an explicit uncertain outcome. A routing decision proposes a next step, and ANANSI's existing policy checks authorize it. The implementation and model choice are design decisions for 2.0. [OpenJEV repository](https://github.com/razorback16/openjev)

## Optional computer scan and Second Brain bubbles

Offer a native desktop scan with the choice of specific folders, selected drives, or the whole computer across eligible locations. Show the scope and exclusions before scanning. The first pass inventories files read-only. The user can opt into AI analysis of supported photos, documents, and other file types for OCR, descriptions, topics, and suggested groups. Prefer local processing and require separate approval for eligible cloud transmission through the existing privacy and cost gates.

Preview the suggested organization before import. In Second Brain, show separate navigable bubbles for groups such as projects, document topics, and photo collections. Let the user rename, split, merge, or exclude bubbles, inspect a source, and review uncertain assignments. Keep source paths and provenance, deduplicate repeated scans, and let users undo imports without changing their original files.

Roll out selected-folder scanning first, then eligible whole-computer scanning once the same controls pass at scale. Skip inaccessible and excluded locations and report what was skipped. Use bounded, resumable jobs with progress, cancellation, and incremental rescans. Scans and AI content analysis are both off by default.

## Conversation-first Ask ANANSI interface

Make Ask ANANSI a simple front door for the 2.0 workflows. A user can say, “Organize my research folder into Second Brain bubbles and assign a Nanzi to summarize it.” ANANSI shows the intended steps, asks for missing information or permissions, lets the user review the scan and import, and shows progress and results in the same conversation. Actions use the shared backend and permission rules. The native desktop host owns filesystem access. The browser can display status and request approved actions through the shared API.

## Acceptance before rollout

- Project and workspace isolation hold across tasks, restart, and revocation. A Nanzi can read only the context granted for the task. Sealed records stay outside inference and tool inputs.
- The same provider privacy, approval, disclosure, and spending gates apply to every task. Background schedules and external actions require explicit opt-in. Tool and computer actions use the approved isolation boundary.
- Verify the create → assign → review → save workflow on the Windows and browser entry points. Include cancellation, denied access, provider failure, restart, and provenance checks before calling the slice ready.
- Verify conversation → approved scan → grouped preview → Second Brain import on a synthetic dataset containing documents, photos, duplicates, unreadable files, and intentionally excluded content. Check cancellation, resume, rescans, manual group corrections, and import undo. Confirm source files are preserved and excluded content never enters model inputs.
- Test the decision layer on ambiguous requests, unsupported actions, and low-confidence grouping. Check that clarification or review happens before an action requiring user input or authorization.

## Enterprise track

[Enterprise Workspaces](post-v1-enterprise-workspaces.md) extends the same Ask ANANSI and Nanzi experience to institution-managed deployment with department isolation, approved internal models, and a separate audit boundary. Start with a read-only institutional pilot after 1.0.

The [master specification](../docs/MASTER_SPEC.md) defines the original release baseline. This addendum records the post-v1 scope and sequencing in the [roadmap](../docs/ROADMAP.md#first-update-after-v1).
