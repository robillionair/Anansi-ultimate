import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import { IntentChoice, Onboarding, PrivacyReview, intents } from '../src/Onboarding';
import { Navigation, plannedGroups } from '../src/Navigation';
import { Settings } from '../src/Settings';
const workspace={contractVersion:1,schemaVersion:2,workspace:{id:'a'.repeat(32),name:'Synthetic'},settings:{intent:'explore' as const,reducedMotion:true,onboardingComplete:true}};
test('all five intents have labeled native radio controls and one checked choice',()=>{
  assert.equal(intents.length,5);const html=renderToStaticMarkup(<IntentChoice value="invent_product" onChange={()=>{}}/>);
  assert.equal((html.match(/type="radio"/g)||[]).length,5);assert.equal((html.match(/checked=""/g)||[]).length,1);
  for(const intent of intents)assert.ok(html.includes(intent.label));assert.ok(html.includes('<fieldset>'));assert.ok(html.includes('<legend>'));
});
test('navigation has a small active set, complete grouped unavailable tools and no ghost recording action',()=>{
  const html=renderToStaticMarkup(<Navigation view="workspace" hasWorkspace={false} onNavigate={()=>{}}/>);
  assert.ok(html.includes('aria-label="Main navigation"'));assert.ok(html.includes('aria-current="page"'));assert.ok(html.includes('disabled=""'));
  assert.equal((html.match(/<button/g)||[]).length,4);for(const group of plannedGroups)for(const item of group.items)assert.ok(html.includes(item));
  assert.ok(html.includes('not available yet'));assert.ok(html.includes('separate consent'));
});
test('onboarding explains actual privacy limits and cannot bypass review using its disabled action',()=>{
  const html=renderToStaticMarkup(<Onboarding workspace={workspace} intent="explore" busy={false} onFinish={()=>{}}/>);
  assert.ok(html.includes('I have reviewed these privacy limits'));assert.ok(html.includes('disabled=""'));assert.ok(html.includes('type="checkbox"'));
  const privacy=renderToStaticMarkup(<PrivacyReview/>);assert.ok(privacy.includes('not an encrypted vault'));assert.ok(privacy.includes('does not authorize future cloud sharing'));
});
test('settings exposes saved intent and meaningful motion action with privacy disclosure',()=>{
  const html=renderToStaticMarkup(<Settings workspace={workspace} busy={false} onSave={()=>{}}/>);
  assert.ok(html.includes('Turn reduced motion off'));assert.ok(html.includes('Save starting point'));assert.ok(html.includes('Review privacy limits'));assert.ok(html.includes('Synthetic'));
});
