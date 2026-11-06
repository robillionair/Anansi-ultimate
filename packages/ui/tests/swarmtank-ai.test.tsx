import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { parseSwarmtankAiRunBody } from '../src/SwarmtankAi.tsx';

function validAiRun(): Record<string, unknown> {
  return {
    format: 'anansi-swarmtank-ai-run',
    version: 1,
    runId: 'swarmtank-ai-abc',
    workspaceId: 'ws-1',
    projectId: 'proj-1',
    input: {
      description: 'A scheduling tool.',
      pricing: '$49 per month',
      priceAmount: 49,
      segment: 'small agencies',
      industry: 'services',
      features: ['automated reports'],
      valueProposition: 'Save time.',
    },
    market: {
      profileId: 'EE',
      profileVersion: 1,
      territoryCode: 'EE',
      territoryName: 'Estonia',
      custom: false,
    },
    seed: 'seed-1',
    agents: [
      {
        id: 'agent-1',
        personaId: 'persona-001',
        name: 'Ava',
        role: 'Technical founder',
        goal: 'Decide whether the product is worth buying.',
        constraints: ['Budget 1000.'],
        language: 'en',
      },
    ],
    scheduledEvents: [],
    rounds: [
      {
        round: 1,
        actions: [
          {
            agentId: 'agent-1',
            round: 1,
            action: 'recommend',
            rationale: 'The price fits and the value is clear.',
            status: 'ok',
            model: 'ollama/llama-x',
            memory: { maxEvents: 5, maxChars: 4000, observedEventIds: [], observedInteractionIds: [] },
          },
        ],
        eventsDue: [],
      },
    ],
    status: 'complete',
    modelProvider: 'ollama',
    model: 'llama-x',
    locality: 'local',
    caps: {
      maxAgents: 3,
      maxRounds: 2,
      maxCalls: 6,
      maxOutputTokens: 512,
      maxTotalTokens: 20000,
      memoryMaxEvents: 5,
      memoryMaxChars: 4000,
    },
    callsMade: 1,
    tokensUsed: 145,
    assumptions: ['Outcomes are conditional simulation results.'],
    contentHash: 'a'.repeat(64),
    createdMs: 123,
  };
}

describe('parseSwarmtankAiRunBody', () => {
  it('parses a valid AI run body', () => {
    const parsed = parseSwarmtankAiRunBody(JSON.stringify(validAiRun()));
    assert.ok(parsed !== null);
    assert.equal(parsed.format, 'anansi-swarmtank-ai-run');
    assert.equal(parsed.status, 'complete');
    assert.equal(parsed.rounds.length, 1);
    assert.equal(parsed.rounds[0].actions[0].action, 'recommend');
  });

  it('parses a failed action (no valid action, recorded failure)', () => {
    const doc = validAiRun();
    (doc.rounds as Record<string, unknown>[])[0].actions = [
      {
        agentId: 'agent-1',
        round: 1,
        status: 'failed',
        failure: 'Malformed model output: Model output is not valid JSON.',
        rawOutput: 'not json',
        model: 'ollama/llama-x',
        memory: { maxEvents: 5, maxChars: 4000, observedEventIds: [], observedInteractionIds: [] },
      },
    ];
    const parsed = parseSwarmtankAiRunBody(JSON.stringify(doc));
    assert.ok(parsed !== null);
    assert.equal(parsed.rounds[0].actions[0].status, 'failed');
    assert.equal(parsed.rounds[0].actions[0].action, undefined);
  });

  it('fails closed on tampered bodies', () => {
    const wrongFormat = validAiRun();
    wrongFormat.format = 'anansi-swarmtank-run';
    assert.equal(parseSwarmtankAiRunBody(JSON.stringify(wrongFormat)), null);

    const badStatus = validAiRun();
    badStatus.status = 'mysterious';
    assert.equal(parseSwarmtankAiRunBody(JSON.stringify(badStatus)), null);

    const badAction = validAiRun();
    ((badAction.rounds as Record<string, unknown>[])[0].actions as Record<string, unknown>[])[0].action = 'buy';
    assert.equal(parseSwarmtankAiRunBody(JSON.stringify(badAction)), null);

    const badHash = validAiRun();
    badHash.contentHash = 'short';
    assert.equal(parseSwarmtankAiRunBody(JSON.stringify(badHash)), null);

    const extraField = validAiRun();
    extraField.surprise = true;
    assert.equal(parseSwarmtankAiRunBody(JSON.stringify(extraField)), null);

    assert.equal(parseSwarmtankAiRunBody('not json'), null);
  });
});
