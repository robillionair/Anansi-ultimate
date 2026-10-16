// Generated from Rust serde DTOs. Run pnpm contracts:generate.
export type ErrorCode =
  | "INVALID_REQUEST"
  | "UNSUPPORTED_CONTRACT"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "INVALID_WORKSPACE"
  | "WORKSPACE_EXISTS"
  | "WORKSPACE_NOT_FOUND"
  | "STORAGE_UNAVAILABLE"
  | "UNSUPPORTED_SCHEMA"
  | "CORRUPT_STORAGE"
  | "INVALID_MEMORY"
  | "MEMORY_NOT_FOUND"
  | "REVISION_CONFLICT"
  | "CONTENT_INTEGRITY"
  | "VAULT_UNAVAILABLE";

export interface CommandError {
  code: ErrorCode;
  message: string;
}
export interface CreateWorkspaceRequest {
  contractVersion: number;
  name: string;
}
export type Privacy = "sealed" | "private" | "standard" | "public";

export interface MemoryDetails {
  body: string;
  contractVersion: number;
  memory: MemoryMetadata;
}
export interface MemoryMetadata {
  confidence?: number | null;
  createdAt: string;
  eventDate?: string | null;
  generatorModel?: string | null;
  id: string;
  kind: string;
  privacy: Privacy;
  projectId?: string | null;
  provenance: string;
  relationships: string[];
  revisionId: string;
  sourceReferences: string[];
  status: string;
  supersedes?: string | null;
  title: string;
  updatedAt: string;
  validFrom?: string | null;
  validUntil?: string | null;
  validationStatus: string;
  version: number;
  workspaceId: string;
}
export interface MemoryDraft {
  body: string;
  confidence?: number | null;
  eventDate?: string | null;
  privacy?: "sealed" | "private" | "standard" | "public";
  sourceReferences?: string[];
  title: string;
  validFrom?: string | null;
  validUntil?: string | null;
}

export interface MemoryList {
  contractVersion: number;
  memories: MemoryMetadata[];
  nextOffset?: number | null;
}
export interface MemoryMetadata {
  confidence?: number | null;
  createdAt: string;
  eventDate?: string | null;
  generatorModel?: string | null;
  id: string;
  kind: string;
  privacy: Privacy;
  projectId?: string | null;
  provenance: string;
  relationships: string[];
  revisionId: string;
  sourceReferences: string[];
  status: string;
  supersedes?: string | null;
  title: string;
  updatedAt: string;
  validFrom?: string | null;
  validUntil?: string | null;
  validationStatus: string;
  version: number;
  workspaceId: string;
}
export interface MemoryListRequest {
  contractVersion: number;
  id?: string | null;
  limit?: number | null;
  offset?: number;
  workspaceId: string;
}

export interface MemoryMetadata {
  confidence?: number | null;
  createdAt: string;
  eventDate?: string | null;
  generatorModel?: string | null;
  id: string;
  kind: string;
  privacy: Privacy;
  projectId?: string | null;
  provenance: string;
  relationships: string[];
  revisionId: string;
  sourceReferences: string[];
  status: string;
  supersedes?: string | null;
  title: string;
  updatedAt: string;
  validFrom?: string | null;
  validUntil?: string | null;
  validationStatus: string;
  version: number;
  workspaceId: string;
}
export interface MemoryReadRequest {
  contractVersion: number;
  id: string;
  revisionId?: string | null;
  workspaceId: string;
}
export interface MemoryWriteRequest {
  confirmPrivacyChange?: boolean;
  contractVersion: number;
  draft: MemoryDraft;
  expectedRevision?: string | null;
  id?: string | null;
  workspaceId: string;
}
export interface MemoryDraft {
  body: string;
  confidence?: number | null;
  eventDate?: string | null;
  privacy?: "sealed" | "private" | "standard" | "public";
  sourceReferences?: string[];
  title: string;
  validFrom?: string | null;
  validUntil?: string | null;
}
export interface OpenWorkspaceRequest {
  contractVersion: number;
  id: string;
}
export interface Readiness {
  apiVersion?: string | null;
  applicationName: string;
  applicationVersion: string;
  contractVersion: number;
  coreVersion: string;
  operatingSystem: string;
  uptimeMs: number;
}
export interface ReadinessRequest {
  contractVersion: number;
}
export type Intent = "second_memory" | "invent_product" | "build_application" | "connect_ai" | "explore";

export interface UpdateSettingsRequest {
  contractVersion: number;
  id: string;
  intent?: Intent | null;
  onboardingComplete?: boolean | null;
  reducedMotion: boolean;
}

export interface WorkspaceDetails {
  contractVersion: number;
  schemaVersion: number;
  settings: WorkspaceSettings;
  workspace: Workspace;
}
export interface WorkspaceSettings {
  intent: Intent;
  onboardingComplete: boolean;
  reducedMotion: boolean;
}
export interface Workspace {
  id: string;
  name: string;
}
export interface WorkspaceList {
  contractVersion: number;
  workspaces: Workspace[];
}
export interface Workspace {
  id: string;
  name: string;
}
