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
  | "CORRUPT_STORAGE";

export interface CommandError {
  code: ErrorCode;
  message: string;
}
export interface CreateWorkspaceRequest {
  contractVersion: number;
  name: string;
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
