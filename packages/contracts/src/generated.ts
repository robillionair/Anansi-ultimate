// Generated from Rust serde DTOs. Run pnpm contracts:generate.
export type ErrorCode = "INVALID_REQUEST" | "UNSUPPORTED_CONTRACT" | "UNAUTHORIZED" | "FORBIDDEN" | "NOT_FOUND";

export interface CommandError {
  code: ErrorCode;
  message: string;
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
