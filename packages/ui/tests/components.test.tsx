import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import { IntentChoice, Onboarding, PrivacyReview, intents } from '../src/Onboarding';
import { Navigation, plannedGroups } from '../src/Navigation';
import { Settings } from '../src/Settings';
import { Projects, IdeaSetView, EvaluationView } from '../src/Projects';
import { Tasks } from '../src/Tasks';
import { People } from '../src/People';
import { KnowledgeRecords } from '../src/KnowledgeRecords';
import { Jobs } from '../src/Jobs';
import { DeliverablePanel, DeliverableView, ForgeBuildPanel, ForgeRevisePanel, SourcePackageView, SourceReviewPanel, SourceReviewView, composeRevisionInstructions, parseReviewBody } from '../src/Forge';
import {Backups} from '../src/Backups';
import {Search} from '../src/Search';
import {LocalModels} from '../src/LocalModels';
import {AskAnansi} from '../src/AskAnansi';
import {Vault} from '../src/Vault';

test('search states its title-only scope, limits and explicit local submission',()=>{
  const html=renderToStaticMarkup(<Search client={{} as any} workspaceId={'a'.repeat(32)}/>);
  assert.match(html,/Search saved titles/);assert.match(html,/Bodies and semantic search are not included/);assert.match(html,/10,000/);assert.match(html,/Nothing is sent/);assert.match(html,/Search titles/);assert.doesNotMatch(html,/Search result revision/);
});

test('search renders mode selector with title, full body, and hybrid semantic options',()=>{
  const html=renderToStaticMarkup(<Search client={{} as any} workspaceId={'a'.repeat(32)}/>);
  assert.match(html,/Search mode/);
  assert.match(html,/Title only/);
  assert.match(html,/Full-body/);
  assert.match(html,/Hybrid semantic/);
  assert.match(html,/value="title"/);
  assert.match(html,/value="body"/);
  assert.match(html,/value="hybrid"/);
});


test('backup creation and restore expose real plaintext/destination/retention consent',()=>{
  const html=renderToStaticMarkup(<Backups client={{} as any} workspaceId={'a'.repeat(32)}/>);
  assert.match(html,/Create backup/);assert.match(html,/disabled=""/);assert.match(html,/plaintext/);assert.match(html,/Keep all versions/);assert.match(html,/Runs only while/);assert.doesNotMatch(html,/Restore verified backup/);
});

test('export explains plaintext privacy, selection boundaries and real resource limits',()=>{
  const html=renderToStaticMarkup(<Jobs client={{} as any} workspaceId={'a'.repeat(32)} native={false}/>);
  assert.match(html,/Sealed content is refused/);assert.match(html,/10,000 records/);assert.match(html,/256 MiB/);assert.match(html,/Project and its current members/);assert.match(html,/Selected records/);assert.doesNotMatch(html,/Download complete export/);
});

const workspace = {
  contractVersion: 1,
  schemaVersion: 5,
  workspace: { id: 'a'.repeat(32), name: 'Synthetic' },
  settings: { intent: 'explore' as const, reducedMotion: true, onboardingComplete: true },
};

const mockClient: any = {
  listMemories: async () => ({ contractVersion: 1, memories: [] }),
  readMemory: async () => ({
    contractVersion: 1,
    memory: {
      id: 'm1',
      workspaceId: 'ws1',
      kind: 'memory',
      revisionId: 'rev1',
      version: 1,
      title: 'Mock record',
      createdAt: '2026-10-17T00:00:00Z',
      updatedAt: '2026-10-17T00:00:00Z',
      status: 'proposed',
      privacy: 'private',
      provenance: 'user_authored',
      sourceReferences: [],
      validationStatus: 'unreviewed',
      relationships: [],
    },
    body: 'Sample body',
  }),
  listRelationships: async () => ({ contractVersion: 1, relationships: [] }),
  aiDiscovery: async () => ({
    contractVersion: 1,
    downloadConsentRequired: true,
    hardware: {
      architecture: 'x86_64',
      logicalCpuCores: 8,
      totalRamBytes: 16 * 1024 * 1024 * 1024,
      availableRamBytes: 10 * 1024 * 1024 * 1024,
      freeStorageBytes: 100 * 1024 * 1024 * 1024,
      gpuName: 'NVIDIA GeForce GTX 1050',
      vramBytes: 2 * 1024 * 1024 * 1024,
    },
    runtimes: [
      {
        endpoint: 'http://127.0.0.1:11434',
        kind: 'ollama',
        status: 'available',
        models: ['llama3.1:8b'],
        version: '0.3.12',
      },
    ],
    recommendations: [
      {
        modelTag: 'llama3.1:8b-instruct-q4_K_M',
        parameterSize: '8B',
        quantization: 'Q4_K_M',
        minimumRamBytes: 8 * 1024 * 1024 * 1024,
        description: 'Balanced general reasoning',
        fitLevel: 'recommended',
      },
    ],
  }),
};

test('all five intents have labeled native radio controls and one checked choice', () => {
  assert.equal(intents.length, 5);
  const html = renderToStaticMarkup(<IntentChoice value="invent_product" onChange={() => {}} />);
  assert.equal((html.match(/type="radio"/g) || []).length, 5);
  assert.equal((html.match(/checked=""/g) || []).length, 1);
  for (const intent of intents) assert.ok(html.includes(intent.label));
  assert.ok(html.includes('<fieldset>'));
  assert.ok(html.includes('<legend>'));
});

test('navigation has a small active set, complete grouped unavailable tools and no ghost recording action', () => {
  const html = renderToStaticMarkup(<Navigation view="workspace" hasWorkspace={false} onNavigate={() => {}} />);
  assert.ok(html.includes('aria-label="Main navigation"'));
  assert.ok(html.includes('aria-current="page"'));
  assert.ok(html.includes('disabled=""'));
  assert.equal((html.match(/<button/g) || []).length, 16);
  assert.ok(!plannedGroups.some(group=>group.items.includes('Search')));
  assert.ok(!plannedGroups.some(group=>group.items.includes('Product Forge')));
  assert.match(html,/disabled=""[^>]*>Ideas &amp; Product Forge<\/button>/);
  assert.doesNotMatch(renderToStaticMarkup(<Navigation view="projects" hasWorkspace={true} onNavigate={()=>{}}/>),/disabled=""[^>]*>Ideas &amp; Product Forge<\/button>/);
  assert.ok(!plannedGroups.some(group=>group.items.includes('Local Models')));
  assert.ok(!plannedGroups.some(group=>group.items.includes('Ask ANANSI')));
  assert.match(renderToStaticMarkup(<Navigation view="search" hasWorkspace={true} onNavigate={()=>{}}/>),/aria-current="page"[^>]*>Search<\/button>/);
  assert.match(renderToStaticMarkup(<Navigation view="models" hasWorkspace={true} onNavigate={()=>{}}/>),/aria-current="page"[^>]*>Local Models<\/button>/);
  assert.match(renderToStaticMarkup(<Navigation view="ask" hasWorkspace={true} onNavigate={()=>{}}/>),/aria-current="page"[^>]*>Ask ANANSI<\/button>/);
  assert.match(renderToStaticMarkup(<Navigation view="providers" hasWorkspace={true} onNavigate={()=>{}}/>),/aria-current="page"[^>]*>AI Providers<\/button>/);
  for (const group of plannedGroups) for (const item of group.items) assert.ok(html.includes(item));
  assert.ok(html.includes('not available yet'));
  assert.ok(html.includes('separate consent'));
});

test('onboarding explains actual privacy limits and cannot bypass review using its disabled action', () => {
  const html = renderToStaticMarkup(<Onboarding workspace={workspace} intent="explore" busy={false} onFinish={() => {}} />);
  assert.ok(html.includes('I have reviewed these privacy limits'));
  assert.ok(html.includes('disabled=""'));
  assert.ok(html.includes('type="checkbox"'));
  const privacy = renderToStaticMarkup(<PrivacyReview />);
  assert.ok(privacy.includes('not an encrypted vault'));
  assert.ok(privacy.includes('does not authorize future cloud sharing'));
  assert.match(privacy,/Onboarding makes no AI or cloud requests/);
  assert.match(privacy,/Optional AI tasks send their inputs and selected context/);
  assert.match(privacy,/requires separate native setup and authorization/);
  assert.doesNotMatch(privacy,/This edition makes no AI|vault are not available yet/);
  const choices=renderToStaticMarkup(<IntentChoice value="explore" onChange={()=>{}}/>);
  assert.match(choices,/Generated code does not run here/);
  assert.doesNotMatch(choices,/connections are not available yet|Forge is still in development/);
});

test('settings exposes saved intent and meaningful motion action with privacy disclosure', () => {
  const html = renderToStaticMarkup(<Settings workspace={workspace} busy={false} onSave={() => {}} />);
  assert.ok(html.includes('Turn reduced motion off'));
  assert.ok(html.includes('Save starting point'));
  assert.ok(html.includes('Review privacy limits'));
  assert.ok(html.includes('Synthetic'));
});

test('projects view renders heading, creation form and accessible list', () => {
  const html = renderToStaticMarkup(<Projects client={mockClient} workspaceId="ws1" />);
  assert.ok(html.includes('Projects</h2>'));
  assert.ok(html.includes('Create a project'));
  assert.ok(html.includes('Project title'));
  assert.ok(html.includes('aria-label="Saved projects"'));
});

test('tasks view renders heading, filter buttons, project selector and completion controls', () => {
  const html = renderToStaticMarkup(<Tasks client={mockClient} workspaceId="ws1" />);
  assert.ok(html.includes('Tasks</h2>'));
  assert.ok(html.includes('Create a task'));
  assert.ok(html.includes('Associated project'));
  assert.ok(html.includes('Due date'));
  assert.ok(html.includes('Mark as completed'));
});

test('people view renders contact management interface and accessible form', () => {
  const html = renderToStaticMarkup(<People client={mockClient} workspaceId="ws1" />);
  assert.ok(html.includes('People</h2>'));
  assert.ok(html.includes('Add a person'));
  assert.ok(html.includes('Full name'));
  assert.ok(html.includes('Role, notes &amp; bio'));
});

test('knowledge records view renders kind selector, filter and entity editor', () => {
  const html = renderToStaticMarkup(<KnowledgeRecords client={mockClient} workspaceId="ws1" />);
  assert.ok(html.includes('Knowledge Records</h2>'));
  assert.ok(html.includes('Filter by kind:'));
  assert.ok(html.includes('Record kind'));
  assert.ok(html.includes('Create a knowledge record'));
});

test('local models view renders hardware profile, discovered runtimes, and explicit consent callout', () => {
  const html = renderToStaticMarkup(<LocalModels client={mockClient} />);
  assert.ok(html.includes('System Capability &amp; Hardware Profile'));
  assert.ok(html.includes('Detected Local AI Runtimes'));
  assert.ok(html.includes('Recommended Local Models for Your Hardware'));
  assert.ok(html.includes('Automatic background model downloads and unprompted cloud fallbacks are disabled'));
  assert.ok(html.includes('Add Custom Loopback Endpoint'));
  assert.ok(html.includes('Re-probe System'));
});

test('ask anansi view renders prompt input, local privacy disclosure, and submission controls', () => {
  const html = renderToStaticMarkup(<AskAnansi client={mockClient} workspaceId="ws1" />);
  assert.ok(html.includes('Ask ANANSI</h2>'));
  assert.ok(html.includes('Ask questions grounded strictly in your saved workspace memories'));
  assert.ok(html.includes('Zero telemetry, zero cloud data transfer'));
  assert.ok(html.includes('id="ask-model"'));
  assert.ok(html.includes('id="ask-embedding-model"'));
  assert.ok(html.includes('id="ask-prompt"'));
  assert.ok(html.includes('maxLength="500"'));
  assert.ok(html.includes('Ask Question'));
});

test('vault view exposes native and paired-browser controls plus encrypted backup/export/restore without password inputs', () => {
  const desktopHtml = renderToStaticMarkup(<Vault workspaceId={'a'.repeat(32)} client={mockClient} native={true} />);
  assert.ok(desktopHtml.includes('Set up vault'));
  assert.ok(desktopHtml.includes('Seal using a verified plaintext backup'));
  assert.ok(desktopHtml.includes('Encrypted vault backup, export, and fresh restore'));
  assert.ok(desktopHtml.includes('Create ciphertext backup'));
  assert.ok(desktopHtml.includes('Create encrypted export'));
  assert.ok(desktopHtml.includes('Restore to new sealed workspace'));
  assert.doesNotMatch(desktopHtml, /type="password"/);

  const browserHtml = renderToStaticMarkup(<Vault workspaceId={'a'.repeat(32)} client={mockClient} native={false} />);
  assert.ok(!browserHtml.includes('Set up vault'));
  assert.ok(browserHtml.includes('Paired browser requests name this origin and workspace'));
  assert.ok(browserHtml.includes('Unlock'));
  assert.ok(browserHtml.includes('Lock'));
  assert.ok(browserHtml.includes('Verify authorized read'));
  assert.ok(browserHtml.includes('Create ciphertext backup'));
  assert.ok(browserHtml.includes('Restore to new sealed workspace'));
  assert.doesNotMatch(browserHtml, /type="password"/);
});

import {Providers, DisclosurePanel, deriveLocality} from '../src/Providers';

test('providers screen renders the full workflow: catalog, owner-trust copy, credential reference (never a secret field), zero-cost lock, routing modes, usage',()=>{
  const html=renderToStaticMarkup(<Providers client={{providersCatalog:async()=>({contractVersion:1,providers:[]}),providersUsage:async()=>({contractVersion:1,totalRequests:0,totalInputTokens:0,totalOutputTokens:0,totalEstimatedCostMicroUsd:0,reservedRequests:0,reservedTokens:0,reservedEstimatedCostMicroUsd:0,providers:[]}),providersSessionAccess:async()=>({contractVersion:1,pending:[],grants:[]})} as any} workspaceId="ws1"/>);
  assert.match(html,/Provider catalog/);
  assert.match(html,/Configure provider/);
  assert.match(html,/owner operations/);
  assert.match(html,/credential reference/i);
  assert.match(html,/operating system keyring/);
  // The credential input is a reference field, never a password box.
  assert.match(html,/Credential reference \(OS keyring account\)/);
  assert.doesNotMatch(html,/type="password"/);
  assert.match(html,/Locality is derived from the endpoint by the backend/);
  assert.match(html,/Zero-Cost Lock/);
  assert.match(html,/Authorize paid dispatch for this model/);
  assert.match(html,/Usage/);
  assert.match(html,/Credential setup/);
  assert.match(html,/Set credential \(native prompt\)/);
  assert.match(html,/trusted native prompt/);
  assert.match(html,/Browser session access/);
  assert.match(html,/cannot approve itself/);
  for(const m of ['auto','autoFree','private','fast','deep','coding','vision','chooseModel'])assert.match(html,new RegExp(`value="${m}"`),`routing mode ${m}`);
});

test('providers locality derivation mirrors the backend rule',()=>{
  assert.equal(deriveLocality('http://127.0.0.1:11434'),'local');
  assert.equal(deriveLocality('http://localhost:1234'),'local');
  assert.equal(deriveLocality('http://[::1]:8080'),'local');
  assert.equal(deriveLocality('https://api.openai.com/v1'),'cloud');
  assert.equal(deriveLocality('not a url'),'cloud');
});

test('disclosure panel renders exactly what ANANSI sent, including attempts and usage',()=>{
  const html=renderToStaticMarkup(<DisclosurePanel disclosure={{provider:'ollama',model:'llama-x',endpointHost:'127.0.0.1:11434',local:true,costClass:'free',routingMode:'auto',routingReason:'only eligible candidate',messageCount:1,segmentCount:2,redactionCount:1,estimatedInputTokens:9,usage:{inputTokens:11,outputTokens:7},attempts:[{provider:'ollama',model:'llama-x',outcome:'succeeded'}]} as any}/>);
  assert.match(html,/View what ANANSI sent/);
  assert.match(html,/127\.0\.0\.1:11434/);
  assert.match(html,/Redactions/);
  assert.match(html,/11 in \/ 7 out/);
  assert.match(html,/ollama\/llama-x: succeeded/);
});

test('projects screen renders its identity and editor statically',()=>{
  const html=renderToStaticMarkup(<Projects client={{} as any} workspaceId={'a'.repeat(32)}/>);
  assert.match(html,/Projects/);assert.match(html,/New project/);assert.match(html,/Project title/);
});

test('idea set view shows the first three hypotheses, honest labels and disabled future actions', () => {
  const concept = (name: string) => ({
    name, problem: 'p', targetCustomer: 'c', solution: 's', whyUseful: 'w',
    differentiationHypothesis: 'h', revenueModel: 'r', engineeringComplexity: 'low',
    implementationCost: 'about a week', keyRisks: ['risk'], requiredValidation: 'interviews',
    minimumViableVersion: 'mvp',
  });
  const html = renderToStaticMarkup(
    <IdeaSetView ideaSet={{ topic: 'harbor', concepts: ['A', 'B', 'C', 'D', 'E'].map(concept) }} />,
  );
  assert.match(html, /Model-generated hypotheses/);
  assert.match(html, /not researched facts/);
  assert.match(html, /1\. A/);
  assert.match(html, /3\. C/);
  assert.doesNotMatch(html, /4\. D/, 'only three concepts are visible before expansion');
  assert.match(html, /Show all 5 concepts/);
  assert.match(html, /Compare/);
  assert.match(html, /disabled/);
});

test('evaluation view renders the rubric snapshot, provenance and not-assessed state', () => {
  const evaluation = {
    format: 'anansi-evaluation',
    sourceArtifactId: 'src123',
    sourceVersion: 2,
    rubricArtifactId: 'rub456',
    rubricVersion: 1,
    rubric: {
      name: 'Standard evaluation rubric',
      dimensions: [
        { key: 'problem-clarity', label: 'Problem clarity', description: 'How clear.', weight: 1 },
        { key: 'mvp-feasibility', label: 'MVP feasibility', description: 'How feasible.', weight: 1 },
      ],
    },
    concepts: [{
      name: 'Alpha', problem: 'p', targetCustomer: 'c', solution: 's', whyUseful: 'w',
      differentiationHypothesis: 'h', revenueModel: 'r', engineeringComplexity: 'low',
      implementationCost: 'a week', keyRisks: ['risk'], requiredValidation: 'interviews',
      minimumViableVersion: 'mvp',
    }],
    conceptIndices: [0],
    assessments: [{ conceptIndex: 0, dimension: 'problem-clarity', score: 4, rationale: 'Sharp.' }],
    summary: [{ conceptIndex: 0, weightedScore: 4, assessedDimensions: 1, totalDimensions: 2 }],
    note: 'User-entered assessments. Not market evidence.',
  };
  const html = renderToStaticMarkup(<EvaluationView evaluation={evaluation} />);
  assert.match(html, /src123/);
  assert.match(html, /version 2/);
  assert.match(html, /rub456/);
  assert.match(html, /Standard evaluation rubric/);
  assert.match(html, /do not change this evaluation/);
  assert.match(html, /4 \/ 5/);
  assert.match(html, /MVP feasibility: not assessed/);
  assert.match(html, /assessed 1 of 2 dimensions/);
  assert.match(html, /Not market evidence/);
});

test('forge build panel exposes an explicit per-version approval action, not a save-as-approval claim',()=>{
  const spec = {
    artifactId: 'b'.repeat(32), projectId: 'c'.repeat(32), type: 'specification', version: 2,
    title: 'Product specification: Bean Stock Log', timestamp: '2026-10-26T00:00:00.000Z',
    sourceReferences: [], generator: null, validationStatus: 'validated', privacy: 'private',
    contentHash: 'd'.repeat(64),
  };
  const html=renderToStaticMarkup(<ForgeBuildPanel client={{} as any} workspaceId={'a'.repeat(32)} projectId={'c'.repeat(32)} spec={spec as any} notify={()=>{}} onSaved={()=>{}} />);
  assert.match(html,/Approve this specification version/);
  assert.match(html,/Loading specification approval status/);
  assert.match(html,/Generate development plan &amp; source/);
  assert.doesNotMatch(html,/Specification approved and saved/);
});

test('forge revise panel states the accept/reject bargain and pipeline stages distinguish generated from saved',()=>{
  const pkg = {
    artifactId: 'b'.repeat(32), projectId: 'c'.repeat(32), type: 'source-package', version: 1,
    title: 'Source package: Bean Stock Log', timestamp: '2026-10-26T00:00:00.000Z',
    sourceReferences: [], generator: null, validationStatus: 'validated', privacy: 'private',
    contentHash: 'd'.repeat(64),
  };
  const doc = {
    format: 'anansi-source-package', formatVersion: 1, template: 'static-web-prototype',
    specArtifactId: 'e'.repeat(32), specVersion: 1, sourceArtifactId: 'f'.repeat(32), sourceVersion: 1,
    conceptName: 'Bean Stock Log',
    plan: { summary: 'One page.', steps: ['Write it.'], buildNotes: 'No build: open index.html.', testingNotes: 'ANANSI has not run this code.' },
    files: [
      { path: 'index.html', content: '<title>Bean</title>' },
      { path: 'README.md', content: '# Bean\n' },
    ],
    limitations: ['Prototype source package generated by a model. It has not been built, tested, executed or deployed by ANANSI.'],
  };
  const panel=renderToStaticMarkup(<ForgeRevisePanel client={{} as any} workspaceId={'a'.repeat(32)} projectId={'c'.repeat(32)} pkg={pkg as any} doc={doc as any} notify={()=>{}} onSaved={()=>{}} />);
  assert.match(panel,/Revise this source package/);
  assert.match(panel,/Request revision/);
  assert.match(panel,/stays unchanged until you explicitly accept/);
  assert.match(panel,/never switches specifications/);
  assert.doesNotMatch(panel,/Generated and saved as a project artifact/);
  // Unsaved draft: pipeline stages are generated, not saved.
  const unsaved=renderToStaticMarkup(<SourcePackageView doc={doc as any} />);
  assert.match(unsaved,/Generated — not yet saved as a project artifact/);
  assert.doesNotMatch(unsaved,/Completed — generated and saved/);
  // Saved artifact: pipeline stages are saved.
  const saved=renderToStaticMarkup(<SourcePackageView doc={doc as any} artifact={pkg as any} />);
  assert.match(saved,/Completed — generated and saved as a project artifact/);
  assert.doesNotMatch(saved,/Generated — not yet saved/);
});

test('deliverable panel states the unexecuted bargain and offers all five documents plus model suggestion',()=>{
  const pkg = {
    artifactId: 'b'.repeat(32), projectId: 'c'.repeat(32), type: 'source-package', version: 1,
    title: 'Source package: Bean Stock Log', timestamp: '2026-10-26T00:00:00.000Z',
    sourceReferences: [], generator: null, validationStatus: 'validated', privacy: 'private',
    contentHash: 'd'.repeat(64),
  };
  const doc = {
    format: 'anansi-source-package', formatVersion: 1, template: 'static-web-prototype',
    specArtifactId: 'e'.repeat(32), specVersion: 1, sourceArtifactId: 'f'.repeat(32), sourceVersion: 1,
    conceptName: 'Bean Stock Log',
    plan: { summary: 'One page.', steps: ['Write it.'], buildNotes: 'No build: open index.html.', testingNotes: 'ANANSI has not run this code.' },
    files: [{ path: 'index.html', content: '<title>Bean</title>' }],
    limitations: ['Prototype source package generated by a model.'],
  };
  const panel=renderToStaticMarkup(<DeliverablePanel client={{} as any} workspaceId={'a'.repeat(32)} projectId={'c'.repeat(32)} pkg={pkg as any} doc={doc as any} notify={()=>{}} onSaved={()=>{}} />);
  assert.match(panel,/Prepare a reviewable deliverable/);
  assert.match(panel,/Unexecuted prototype — prepared for review/);
  assert.match(panel,/Export is not publication/);
  assert.match(panel,/Suggest documentation with model/);
  assert.match(panel,/Save deliverable/);
  for (const name of ['architecture.md','usage.md','limitations.md','security.md','changes.md']) {
    assert.match(panel, new RegExp(name.replace('.', '\.')));
  }
  assert.doesNotMatch(panel,/license\.md/);
  assert.doesNotMatch(panel,/deployment\.md/);
});

test('source review panel states the inspection-only bargain and loads the pinned specification',()=>{
  const pkg = {
    artifactId: 'b'.repeat(32), projectId: 'c'.repeat(32), type: 'source-package', version: 1,
    title: 'Source package: Bean Stock Log', timestamp: '2026-10-26T00:00:00.000Z',
    sourceReferences: [], generator: null, validationStatus: 'validated', privacy: 'private',
    contentHash: 'd'.repeat(64),
  };
  const doc = {
    format: 'anansi-source-package', formatVersion: 1, template: 'static-web-prototype',
    specArtifactId: 'e'.repeat(32), specVersion: 1, sourceArtifactId: 'f'.repeat(32), sourceVersion: 1,
    conceptName: 'Bean Stock Log',
    plan: { summary: 'One page.', steps: ['Write it.'], buildNotes: 'No build: open index.html.', testingNotes: 'ANANSI has not run this code.' },
    files: [{ path: 'index.html', content: '<title>Bean</title>' }],
    limitations: ['Prototype source package generated by a model.'],
  };
  const panel=renderToStaticMarkup(<SourceReviewPanel client={{} as any} workspaceId={'a'.repeat(32)} projectId={'c'.repeat(32)} pkg={pkg as any} doc={doc as any} notify={()=>{}} onSaved={()=>{}} onPrepareRevision={()=>{}} />);
  assert.match(panel,/Review this source package/);
  assert.match(panel,/Inspection only/);
  assert.match(panel,/starts no inference and approves nothing/);
  assert.match(panel,/Loading pinned specification/);
  assert.doesNotMatch(panel,/has been tested/);
});

test('source review view pins versions, shows findings and never claims runtime behavior',()=>{
  const doc = {
    format: 'anansi-source-review', formatVersion: 1,
    packageArtifactId: 'b'.repeat(32), packageVersion: 1, packageContentHash: 'd'.repeat(64),
    specArtifactId: 'e'.repeat(32), specVersion: 1, specContentHash: 'd'.repeat(64),
    conceptName: 'Bean Stock Log',
    findings: [
      { criterionIndex: 0, criterion: 'Open index.html and add a bean', status: 'satisfied', note: 'Add flow present in app.js.', fileRefs: ['app.js'] },
      { criterionIndex: 1, criterion: 'Reload keeps entries', status: 'needs-changes', note: 'Reload loses the last entry.', fileRefs: [] },
    ],
    limitations: ['Inspection does not establish runtime behavior.'],
  };
  const artifact = {
    artifactId: 'g'.repeat(32), projectId: 'c'.repeat(32), type: 'source-review', version: 1,
    title: 'Source review: Bean Stock Log', timestamp: '2026-10-26T00:00:00.000Z',
    sourceReferences: [], generator: null, validationStatus: 'validated', privacy: 'private',
    contentHash: 'd'.repeat(64),
  };
  const html=renderToStaticMarkup(<SourceReviewView doc={doc as any} artifact={artifact as any} client={{} as any} workspaceId={'a'.repeat(32)} notify={()=>{}} onSaved={()=>{}} onPrepareRevision={()=>{}} />);
  assert.match(html,/Pins source package artifact/);
  assert.match(html,/Inspection only/);
  assert.match(html,/Reload loses the last entry/);
  assert.match(html,/Status: needs-changes/);
  assert.match(html,/Revise this review/);
  assert.match(html,/Use findings for a revision/);
  assert.doesNotMatch(html,/has been tested/);
  assert.doesNotMatch(html,/certified this code/);
});

test('revision instructions compose only selected findings for inspection',()=>{
  const drafts = [
    { criterionIndex: 0, criterion: 'Open index.html and add a bean', status: 'satisfied', note: '', fileRefsText: '', selected: false },
    { criterionIndex: 1, criterion: 'Reload keeps entries', status: 'needs-changes', note: 'Reload loses the last entry.', fileRefsText: 'app.js', selected: true },
  ];
  const text = composeRevisionInstructions(drafts as any, 'Bean Stock Log', 1);
  assert.match(text,/Revision requested from source review of Bean Stock Log/);
  assert.match(text,/Reload keeps entries/);
  assert.match(text,/Reload loses the last entry/);
  assert.match(text,/app\.js/);
  assert.doesNotMatch(text,/add a bean/);
  assert.equal(composeRevisionInstructions(drafts.map(d => ({ ...d, selected: false })) as any, 'Bean Stock Log', 1), '');
});

test('review body parser accepts only the review format',()=>{
  const doc = {
    format: 'anansi-source-review', formatVersion: 1,
    packageArtifactId: 'b'.repeat(32), packageVersion: 1, packageContentHash: 'd'.repeat(64),
    specArtifactId: 'e'.repeat(32), specVersion: 1, specContentHash: 'd'.repeat(64),
    conceptName: 'Bean Stock Log', findings: [], limitations: [],
  };
  assert.equal(parseReviewBody(JSON.stringify(doc))?.format, 'anansi-source-review');
  assert.equal(parseReviewBody(JSON.stringify({ ...doc, format: 'anansi-deliverable' })), null);
  assert.equal(parseReviewBody('not json'), null);
});

test('deliverable view pins versions, shows origins and never claims publication',()=>{
  const doc = {
    format: 'anansi-deliverable', formatVersion: 1,
    packageArtifactId: 'b'.repeat(32), packageVersion: 1, packageContentHash: 'd'.repeat(64),
    specArtifactId: 'e'.repeat(32), specVersion: 1, specContentHash: 'd'.repeat(64),
    conceptName: 'Bean Stock Log',
    documents: [
      { name: 'architecture.md', content: 'Arch.', origin: 'user' },
      { name: 'verified-facts.md', content: 'Facts: index.html.', origin: 'verified' },
    ],
    status: { executed: false, built: false, tested: false, published: false, productionReady: false },
  };
  const html=renderToStaticMarkup(<DeliverableView doc={doc as any} />);
  assert.match(html,/Pins source package artifact/);
  assert.match(html,/Unexecuted prototype — prepared for review/);
  assert.match(html,/verified/);
  assert.match(html,/not production-ready/);
  assert.doesNotMatch(html,/Published/);
});
