ALTER TABLE workspace_settings ADD COLUMN intent TEXT NOT NULL DEFAULT 'explore'
    CHECK (intent IN ('second_memory', 'invent_product', 'build_application', 'connect_ai', 'explore'));
ALTER TABLE workspace_settings ADD COLUMN onboarding_complete INTEGER NOT NULL DEFAULT 0
    CHECK (onboarding_complete IN (0, 1));
