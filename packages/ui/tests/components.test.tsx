import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import { IntentChoice, Onboarding, PrivacyReview, intents } from '../src/Onboarding';
import { Navigation, plannedGroups } from '../src/Navigation';
import { Settings } from '../src/Settings';
import { Projects } from '../src/Projects';
import { Tasks } from '../src/Tasks';
import { People } from '../src/People';
import { KnowledgeRecords } from '../src/KnowledgeRecords';
import { Jobs } from '../src/Jobs';
import {Backups} from '../src/Backups';

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
  assert.equal((html.match(/<button/g) || []).length, 11);
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
