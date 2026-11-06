// ---------------------------------------------------------------------------
// SwarmTank v2 Checkpoint 1: market selection + profile provenance.
//
// A searchable CLDR territory list (versioned, served by the core), a
// researched profile view (every evidence-backed input with its source
// and date; gaps as editable assumptions), and a Custom assumptions
// state for territories with no researched profile. Geography is
// separate from personality: the profile shapes which buyers appear
// and what they can spend, never a personality trait.
// ---------------------------------------------------------------------------

import { useEffect, useMemo, useState } from 'react';
import type {
  Client,
  MarketAssumption,
  MarketProfile,
  MarketProfileSummary,
  SwarmtankMarket,
  Territory,
} from '@anansi/contracts';

export type MarketSelection = SwarmtankMarket | null;

const CUSTOM_FIELDS: { field: string; label: string; placeholder: string }[] = [
  { field: 'business_language', label: 'Business language', placeholder: 'e.g. English' },
  { field: 'typical_budget', label: 'Typical budget', placeholder: 'e.g. Small monthly budgets; price-sensitive' },
  { field: 'procurement_norm', label: 'Procurement norm', placeholder: 'e.g. Owner decides; short chain' },
  { field: 'buying_authority', label: 'Buying authority', placeholder: 'e.g. Founder / CEO decides' },
];

function customAssumptionsFrom(values: Record<string, string>): MarketAssumption[] {
  return CUSTOM_FIELDS.filter(item => (values[item.field] ?? '').trim() !== '').map(item => ({
    field: item.field,
    value: (values[item.field] ?? '').trim(),
    rationale: 'User-supplied for this territory; no researched profile.',
    editable: true,
  }));
}

export function SwarmtankMarketSelector({
  client,
  value,
  onChange,
}: {
  client: Client;
  value: MarketSelection;
  onChange: (market: MarketSelection) => void;
}) {
  const [territories, setTerritories] = useState<Territory[] | null>(null);
  const [profiles, setProfiles] = useState<MarketProfileSummary[] | null>(null);
  const [cldrVersion, setCldrVersion] = useState('');
  const [loadError, setLoadError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [territoryCode, setTerritoryCode] = useState<string | null>(
    value?.territoryCode ?? null,
  );
  const [profile, setProfile] = useState<MarketProfile | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [customValues, setCustomValues] = useState<Record<string, string>>(() => {
    const out: Record<string, string> = {};
    for (const item of value?.customAssumptions ?? []) out[item.field] = item.value;
    return out;
  });
  const [editedAssumptions, setEditedAssumptions] = useState<Record<string, string>>(() => {
    const out: Record<string, string> = {};
    for (const item of value?.customAssumptions ?? []) out[item.field] = item.value;
    return out;
  });

  useEffect(() => {
    let active = true;
    Promise.all([client.listSwarmtankTerritories(), client.listSwarmtankMarkets()])
      .then(([territoryResponse, marketResponse]) => {
        if (!active) return;
        setTerritories(territoryResponse.territories);
        setCldrVersion(territoryResponse.cldrVersion);
        setProfiles(marketResponse.profiles);
      })
      .catch(() => {
        if (active) setLoadError('Could not load the market list. Reopen SwarmTank to retry.');
      });
    return () => {
      active = false;
    };
  }, [client]);

  const territoryName = useMemo(() => {
    if (!territoryCode) return null;
    return territories?.find(t => t.code === territoryCode)?.name ?? value?.territoryName ?? null;
  }, [territoryCode, territories, value?.territoryName]);

  const profilesForTerritory = useMemo(() => {
    if (!territoryCode || !profiles) return [];
    return profiles.filter(p => p.territoryCode === territoryCode);
  }, [territoryCode, profiles]);

  // When the selected territory has exactly one researched profile with
  // no subdivision, load it. When it has subdivisions, the user picks
  // one below. When it has none, the Custom assumptions state opens.
  useEffect(() => {
    if (!territoryCode || profiles === null) return;
    const matches = profiles.filter(p => p.territoryCode === territoryCode);
    if (matches.length === 1 && !matches[0].subdivision) {
      setProfileLoading(true);
      void client
        .getSwarmtankMarket(matches[0].id)
        .then(result => {
          if (result.researched && result.profile) setProfile(result.profile);
          else setProfile(null);
        })
        .catch(() => setProfile(null))
        .finally(() => setProfileLoading(false));
    } else {
      setProfile(null);
    }
  }, [territoryCode, profiles, client]);

  // Publish the selection upward whenever the territory, profile, or
  // edited assumptions change.
  useEffect(() => {
    if (!territoryCode) {
      onChange(null);
      return;
    }
    if (profile) {
      const edited = Object.entries(editedAssumptions)
        .filter(([, text]) => text.trim() !== '')
        .map(([field, text]) => {
          const original = profile.assumptions.find(a => a.field === field);
          return {
            field,
            value: text.trim(),
            rationale: original
              ? 'Edited by the user; profile default replaced.'
              : 'Added by the user.',
            editable: true,
          } as MarketAssumption;
        });
      onChange({
        profileId: profile.id,
        profileVersion: profile.profileVersion,
        territoryCode: profile.territoryCode,
        territoryName: profile.name,
        custom: false,
        customAssumptions: edited.length > 0 ? edited : undefined,
      });
      return;
    }
    // No researched profile for this territory: Custom assumptions.
    const assumptions = customAssumptionsFrom(customValues);
    onChange({
      territoryCode,
      territoryName: territoryName ?? territoryCode,
      custom: true,
      customAssumptions: assumptions.length > 0 ? assumptions : undefined,
    });
  }, [territoryCode, profile, editedAssumptions, customValues, territoryName, onChange]);

  const filtered = useMemo(() => {
    if (!territories) return [];
    const q = query.trim().toLowerCase();
    if (q === '') return territories;
    return territories.filter(
      t => t.name.toLowerCase().includes(q) || t.code.toLowerCase().includes(q),
    );
  }, [territories, query]);

  const selectTerritory = (code: string) => {
    setTerritoryCode(code);
    setProfile(null);
    setEditedAssumptions({});
    setCustomValues({});
  };

  const clearMarket = () => {
    setTerritoryCode(null);
    setProfile(null);
    setQuery('');
    setEditedAssumptions({});
    setCustomValues({});
  };

  return (
    <section aria-label="Market">
      <h3>Market</h3>
      <p>
        Choose the market this simulation is for. Personas are shaped by the
        market&apos;s buyer roles, company sizes and budget evidence — never
        by nationality. Territory names use Unicode CLDR
        {cldrVersion ? ` ${cldrVersion}` : ''}.
      </p>
      {loadError && <p role="alert">{loadError}</p>}
      {territories === null && !loadError && <p role="status">Loading markets…</p>}
      {territories !== null && (
        <>
          <label htmlFor="swarmtank-market-search">Search country or territory</label>
          <input
            id="swarmtank-market-search"
            type="search"
            value={query}
            onChange={event => setQuery(event.target.value)}
            placeholder="e.g. Estonia, AE, United States"
            autoComplete="off"
          />
          <label htmlFor="swarmtank-market-select">Country or territory</label>
          <select
            id="swarmtank-market-select"
            value={territoryCode ?? ''}
            onChange={event => {
              if (event.target.value === '') clearMarket();
              else selectTerritory(event.target.value);
            }}
          >
            <option value="">No market (unspecified)</option>
            {filtered.map(t => (
              <option key={t.code} value={t.code}>
                {t.name} ({t.code})
                {profiles?.some(p => p.territoryCode === t.code) ? ' — researched profile' : ''}
              </option>
            ))}
          </select>
          {filtered.length === 0 && <p role="status">No territory matches that search.</p>}
          {territoryCode && territoryName && (
            <p>
              Market: <strong>{territoryName}</strong> ({territoryCode})
              <button type="button" onClick={clearMarket}>
                Clear market
              </button>
            </p>
          )}
          {territoryCode && profiles !== null && profilesForTerritory.length > 1 && (
            <fieldset>
              <legend>Choose a researched profile for {territoryName}</legend>
              {profilesForTerritory.map(p => (
                <label key={p.id}>
                  <input
                    type="radio"
                    name="swarmtank-market-profile"
                    checked={profile?.id === p.id}
                    onChange={() => {
                      setProfileLoading(true);
                      void client
                        .getSwarmtankMarket(p.id)
                        .then(result => {
                          if (result.researched && result.profile) setProfile(result.profile);
                        })
                        .catch(() => setProfile(null))
                        .finally(() => setProfileLoading(false));
                    }}
                  />
                  {p.subdivision ?? p.name} (v{p.profileVersion})
                </label>
              ))}
            </fieldset>
          )}
          {territoryCode && profileLoading && <p role="status">Loading profile…</p>}
          {territoryCode && profile && (
            <MarketProfileView
              profile={profile}
              editedAssumptions={editedAssumptions}
              onEditAssumption={(field, text) =>
                setEditedAssumptions(prev => ({ ...prev, [field]: text }))
              }
            />
          )}
          {territoryCode && !profile && !profileLoading && profiles !== null && (
            <section aria-label="Custom assumptions">
              <h4>Custom assumptions</h4>
              <p>
                No researched profile exists for {territoryName}. This run is
                labelled <strong>Custom assumptions</strong> — the values below
                are yours, not evidence. They are stored with the run so a
                reader can tell researched from custom.
              </p>
              {CUSTOM_FIELDS.map(item => (
                <div key={item.field}>
                  <label htmlFor={`swarmtank-custom-${item.field}`}>{item.label}</label>
                  <input
                    id={`swarmtank-custom-${item.field}`}
                    value={customValues[item.field] ?? ''}
                    onChange={event =>
                      setCustomValues(prev => ({ ...prev, [item.field]: event.target.value }))
                    }
                    placeholder={item.placeholder}
                    maxLength={500}
                  />
                </div>
              ))}
            </section>
          )}
        </>
      )}
    </section>
  );
}

function MarketProfileView({
  profile,
  editedAssumptions,
  onEditAssumption,
}: {
  profile: MarketProfile;
  editedAssumptions: Record<string, string>;
  onEditAssumption: (field: string, text: string) => void;
}) {
  return (
    <section aria-label={`Researched profile: ${profile.name}`}>
      <h4>
        Researched profile: {profile.name} (v{profile.profileVersion})
      </h4>
      <p>
        Territory {profile.territoryCode}
        {profile.subdivision ? ` — ${profile.subdivision}` : ''} · Currency{' '}
        {profile.currency} · Languages:{' '}
        {profile.languages.map(l => `${l.name} (${l.role})`).join(', ')} · CLDR{' '}
        {profile.cldrVersion}
      </p>
      <h5>Evidence (each with source and date)</h5>
      <ul>
        {profile.evidence.map(entry => (
          <li key={entry.field}>
            <strong>{entry.field}:</strong> {entry.value}{' '}
            <span>
              — {entry.source.name},{' '}
              <a href={entry.source.url} target="_blank" rel="noreferrer">
                source
              </a>{' '}
              ({entry.date})
            </span>
          </li>
        ))}
      </ul>
      <h5>Assumptions (editable — gaps, not evidence)</h5>
      <ul>
        {profile.assumptions.map(item => (
          <li key={item.field}>
            <label htmlFor={`swarmtank-assumption-${item.field}`}>{item.field}</label>
            <input
              id={`swarmtank-assumption-${item.field}`}
              value={editedAssumptions[item.field] ?? item.value}
              onChange={event => onEditAssumption(item.field, event.target.value)}
              maxLength={3000}
            />
            <p>{item.rationale}</p>
          </li>
        ))}
      </ul>
      <h5>Buyer segments</h5>
      <ul>
        {profile.segments.map(segment => (
          <li key={segment.id}>
            <strong>{segment.label}</strong> ({segment.companySize}) — roles:{' '}
            {segment.roles.join(', ')} — {segment.buyingAuthority}
          </li>
        ))}
      </ul>
    </section>
  );
}
