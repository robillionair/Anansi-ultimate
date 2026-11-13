# installed-workflow-before

````text
node:internal/modules/run_main:107
    triggerUncaughtException(
    ^

ExpectError: expect(locator).toBeVisible() failed

Locator: getByRole('heading', { name: 'Synthetic native workspace', exact: true })
Expected: visible
Timeout: 15000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" getByRole('heading', { name: 'Synthetic native workspace', exact: true }) with timeout 15000ms
  - waiting for getByRole('heading', { name: 'Synthetic native workspace', exact: true })

    at captureRawStack (E:\Anansi\worktrees\review-claim-recovery-final\node_modules\.pnpm\playwright-core@1.64.1\node_modules\playwright-core\lib\coreBundle.js:9282:17)
    at callMatcherAsStep (E:\Anansi\worktrees\review-claim-recovery-final\node_modules\.pnpm\playwright@1.64.1\node_modules\playwright\lib\matchers\expect.js:13316:57)
    at Object.toBeVisible (E:\Anansi\worktrees\review-claim-recovery-final\node_modules\.pnpm\playwright@1.64.1\node_modules\playwright\lib\matchers\expect.js:13308:23)
    at create (E:\Anansi\worktrees\review-claim-recovery-final\scripts\verify-workspace.mjs:49:61)
    at async file:///E:/Anansi/worktrees/review-claim-recovery-final/scripts/verify-workspace.mjs:59:18 {
  matcherResult: {
    message: 'expect(locator).toBeVisible() failed\n' +
      '\n' +
      "Locator: getByRole('heading', { name: 'Synthetic native workspace', exact: true })\n" +
      'Expected: visible\n' +
      'Timeout: 15000ms\n' +
      'Error: element(s) not found\n' +
      '\n' +
      'Call log:\n' +
      `  - Expect "toBeVisible" getByRole('heading', { name: 'Synthetic native workspace', exact: true }) with timeout 15000ms\n` +
      "  - waiting for getByRole('heading', { name: 'Synthetic native workspace', exact: true })\n",
    pass: false,
    actual: undefined,
    name: 'toBeVisible',
    expected: 'visible',
    log: [
      `  - Expect "toBeVisible" getByRole('heading', { name: 'Synthetic native workspace', exact: true }) with timeout 15000ms`,
      "  - waiting for getByRole('heading', { name: 'Synthetic native workspace', exact: true })"
    ],
    timeout: 15000,
    ariaSnapshot: '- link "Skip to main content":\n' +
      '  - /url: "#main"\n' +
      '- complementary:\n' +
      '  - link "A ANANSI ULTIMATE":\n' +
      '    - /url: "#main"\n' +
      '  - navigation "Main navigation":\n' +
      '    - paragraph: HOME\n' +
      '    - button "Dashboard"\n' +
      '    - button "Ask ANANSI"\n' +
      '    - paragraph: CREATE\n' +
      '    - button "Ideas & Product Forge"\n' +
      '    - paragraph: WORK\n' +
      '    - button "Projects"\n' +
      '    - button "Tasks"\n' +
      '    - button "People"\n' +
      '    - paragraph: KNOWLEDGE\n' +
      '    - button "Search"\n' +
      '    - button "Memory"\n' +
      '    - button "Notes"\n' +
      '    - button "Records"\n' +
      '    - button "Documents"\n' +
      '    - paragraph: SYSTEM\n' +
      '    - button "Jobs and export"\n' +
      '    - button "Backups"\n' +
      '    - button "Local Models"\n' +
      '    - button "AI Providers"\n' +
      '    - button "MCP"\n' +
      '    - button "Settings"\n' +
      '    - button "Connection"\n' +
      '    - group: Explore planned tools\n' +
      '  - paragraph: Windows desktop Development preview\n' +
      '- main:\n' +
      '  - text: YOUR PRIVATE STARTING POINT\n' +
      '  - status: Connected\n' +
      '  - button "Toggle LARP mode": "[ LARP: OFF ]"\n' +
      '  - heading "Dashboard" [level=1]\n' +
      '  - paragraph: Your workspace at a glance — a few meaningful actions, nothing more.\n' +
      '  - region "Workspace overview":\n' +
      '    - heading "Workspace overview" [level=2]\n' +
      '    - paragraph:\n' +
      '      - strong: Synthetic native workspace\n' +
      '      - text: "· Workspace ID:"\n' +
      '      - code: 425f6e16a19f841b777cddb27d760992\n' +
      '    - paragraph:\n' +
      '      - status: Ready · core up 13s\n' +
      '    - term: Memory nodes\n' +
      '    - definition:\n' +
      '      - strong: "0"\n' +
      '    - term: Active notes\n' +
      '    - definition:\n' +
      '      - strong: "0"\n' +
      '    - term: Recorded relationships\n' +
      '    - definition:\n' +
      '      - strong: "0"\n' +
      '    - button "Manage workspace"\n' +
      '  - region "System & storage health":\n' +
      '    - heading "System & storage health" [level=2]\n' +
      '    - paragraph: "Backup: Manual backups only"\n' +
      '    - paragraph: Background queue idle\n' +
      '    - button "Backups"\n' +
      '    - button "Jobs"\n' +
      '  - region "Recent & active projects":\n' +
      '    - heading "Recent & active projects" [level=2]\n' +
      '    - paragraph: No projects yet — create your first project.\n' +
      '  - region "Quick actions":\n' +
      '    - heading "Quick actions" [level=2]\n' +
      '    - paragraph: Four meaningful actions. Everything else is one navigation click away.\n' +
      '    - button "+ New Project"\n' +
      '    - button "Ask ANANSI"\n' +
      '    - button "LEXI-COLLIDER"\n' +
      '    - button "LARP Telemetry Engine Off"\n' +
      '  - strong: Local foundation · In development\n' +
      '  - paragraph: Workspace settings, notes, memory lifecycle, projects, tasks, entities and relationships work today. Workspace backups and restore preview are available. Project-scoped ideas, source revision and documented exports are available with a configured provider. Generated products are not executed, built or deployed here. This development preview is not production-certified.'
  }
}

Node.js v26.7.0
Synthetic installer evidence retained: E:\Anansi\qualification\installer-test-2026-11-13
Exception: E:\Anansi\worktrees\review-claim-recovery-final\scripts\verify-installer.ps1:71
Line |
  71 |  … ASTEXITCODE -ne 0){throw 'Installed native/browser workflow failed.'}
     |                       ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
     | Installed native/browser workflow failed.
````
