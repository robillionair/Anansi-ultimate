// Generated from Rust serde DTOs. Run pnpm contracts:generate.
export interface AiDiscoveryRequest {
  contractVersion: number;
  customEndpoints?: string[] | null;
}
export type FitLevel = "recommended" | "comfortable" | "tight" | "insufficient";
export type RuntimeKind = "ollama" | "lmStudio" | "customLoopback";
export type RuntimeStatus = "available" | "offline" | "unsupported";

export interface AiDiscoveryResponse {
  contractVersion: number;
  downloadConsentRequired: boolean;
  hardware: HardwareProfile;
  recommendations: ModelRecommendation[];
  runtimes: DiscoveredRuntime[];
}
export interface HardwareProfile {
  architecture: string;
  availableRamBytes: number;
  freeStorageBytes: number;
  gpuName?: string | null;
  logicalCpuCores: number;
  totalRamBytes: number;
  vramBytes?: number | null;
}
export interface ModelRecommendation {
  description: string;
  fitLevel: FitLevel;
  minimumRamBytes: number;
  modelTag: string;
  parameterSize: string;
  quantization: string;
}
export interface DiscoveredRuntime {
  endpoint: string;
  kind: RuntimeKind;
  models: string[];
  status: RuntimeStatus;
  version?: string | null;
}
export interface ArtifactChunk {
  bytes: number[];
  contractVersion: number;
}
export interface AskRequest {
  contractVersion: number;
  maxEvidenceItems?: number | null;
  modelTag?: string | null;
  prompt: string;
  runtimeEndpoint?: string | null;
  workspaceId: string;
}
export type AskStatus = "answered" | "insufficientEvidence" | "offline" | "error";

export interface AskResponse {
  answer: string;
  citations: GroundedCitation[];
  contractVersion: number;
  manifest: DisclosureManifest;
  status: AskStatus;
}
export interface GroundedCitation {
  index: number;
  kind: string;
  lineNumber?: number | null;
  memoryId: string;
  sectionHeading?: string | null;
  snippet: string;
  title: string;
}
export interface DisclosureManifest {
  characterCount: number;
  compiledPrompt: string;
  endpoint: string;
  estimatedTokens: number;
  modelTag: string;
  routingReason: string;
  runtimeKind: string;
  sourceMemories: SourceMemoryReference[];
}
export interface SourceMemoryReference {
  id: string;
  kind: string;
  title: string;
}
export interface BackupRequest {
  application: boolean;
  contractVersion: number;
  plaintextConsent: boolean;
  workspaceId: string;
}
export interface BackupSchedule {
  contractVersion: number;
  destination: string;
  enabled: boolean;
  intervalSeconds: number;
  missedRuns: number;
  nextDue?: number | null;
  retention: string;
}
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
  | "INVALID_SEARCH"
  | "MEMORY_NOT_FOUND"
  | "REVISION_CONFLICT"
  | "CONTENT_INTEGRITY"
  | "VAULT_UNAVAILABLE";

export interface CommandError {
  code: ErrorCode;
  message: string;
}
export interface CompiledContext {
  citations: GroundedCitation[];
  contractVersion: number;
  disclosureManifest: DisclosureManifest;
  evidenceChunks: ContextChunk[];
  systemPrompt: string;
  totalCharacters: number;
  totalTokens: number;
}
export interface GroundedCitation {
  index: number;
  kind: string;
  lineNumber?: number | null;
  memoryId: string;
  sectionHeading?: string | null;
  snippet: string;
  title: string;
}
export interface DisclosureManifest {
  characterCount: number;
  compiledPrompt: string;
  endpoint: string;
  estimatedTokens: number;
  modelTag: string;
  routingReason: string;
  runtimeKind: string;
  sourceMemories: SourceMemoryReference[];
}
export interface SourceMemoryReference {
  id: string;
  kind: string;
  title: string;
}
export interface ContextChunk {
  chunkIndex: number;
  content: string;
  kind: string;
  lineNumber?: number | null;
  memoryId: string;
  sectionHeading?: string | null;
  title: string;
  tokenEstimate: number;
}
export interface ContextChunk {
  chunkIndex: number;
  content: string;
  kind: string;
  lineNumber?: number | null;
  memoryId: string;
  sectionHeading?: string | null;
  title: string;
  tokenEstimate: number;
}
export interface ContextCompilationRequest {
  kindFilter?: string | null;
  maxBudgetChars?: number | null;
  modelTag?: string | null;
  runtimeEndpoint?: string | null;
  userPrompt: string;
  workspaceId: string;
}
export type JobAction = "pause" | "resume" | "cancel" | "retry";

export interface ControlJobRequest {
  action: JobAction;
  contractVersion: number;
  expectedVersion: number;
  id: string;
  workspaceId: string;
}
export interface CreateRelationshipRequest {
  contractVersion: number;
  draft: RelationshipDraft;
  workspaceId: string;
}
export interface RelationshipDraft {
  metadata?: {
    [k: string]: unknown;
  };
  predicate: string;
  sourceId: string;
  targetId: string;
}
export interface CreateWorkspaceRequest {
  contractVersion: number;
  name: string;
}
export interface DeleteRelationshipRequest {
  contractVersion: number;
  expectedVersion: number;
  id: string;
  workspaceId: string;
}
export interface DisclosureManifest {
  characterCount: number;
  compiledPrompt: string;
  endpoint: string;
  estimatedTokens: number;
  modelTag: string;
  routingReason: string;
  runtimeKind: string;
  sourceMemories: SourceMemoryReference[];
}
export interface SourceMemoryReference {
  id: string;
  kind: string;
  title: string;
}

export interface DiscoveredRuntime {
  endpoint: string;
  kind: RuntimeKind;
  models: string[];
  status: RuntimeStatus;
  version?: string | null;
}
export type ExportSelection =
  | {
      kind: "workspace";
    }
  | {
      application: boolean;
      kind: "backup";
    }
  | {
      id: string;
      kind: "project";
    }
  | {
      ids: string[];
      kind: "records";
    };

export interface EnqueueExportRequest {
  contractVersion: number;
  selection: ExportSelection;
  workspaceId: string;
}
export interface GroundedCitation {
  index: number;
  kind: string;
  lineNumber?: number | null;
  memoryId: string;
  sectionHeading?: string | null;
  snippet: string;
  title: string;
}
export interface HardwareProfile {
  architecture: string;
  availableRamBytes: number;
  freeStorageBytes: number;
  gpuName?: string | null;
  logicalCpuCores: number;
  totalRamBytes: number;
  vramBytes?: number | null;
}
export interface HybridSearchRequest {
  includeArchived?: boolean;
  kind?: string | null;
  lexicalWeight?: number;
  limit?: number;
  model?: string | null;
  query?: string | null;
  queryVector?: EmbeddingVector | null;
  rrfK?: number;
  semanticWeight?: number;
  workspaceId: string;
}
export interface EmbeddingVector {
  values: number[];
}
export interface HybridSearchResponse {
  contractVersion: number;
  results: HybridSearchResult[];
}
export interface HybridSearchResult {
  kind: string;
  lexicalRank?: number | null;
  lineNumber?: number | null;
  memoryId: string;
  rrfScore: number;
  sectionHeading?: string | null;
  semanticRank?: number | null;
  snippet?: string | null;
  title: string;
}
export interface HybridSearchResult {
  kind: string;
  lexicalRank?: number | null;
  lineNumber?: number | null;
  memoryId: string;
  rrfScore: number;
  sectionHeading?: string | null;
  semanticRank?: number | null;
  snippet?: string | null;
  title: string;
}
export type JobState = "Queued" | "Running" | "Paused" | "Completed" | "Failed" | "Cancelled";

export interface Job {
  artifactBytes?: number | null;
  artifactHash?: string | null;
  artifactId?: string | null;
  checkpoint: string;
  checkpointVersion: number;
  createdAt: string;
  id: string;
  log: string;
  maxBytes: number;
  maxRecords: number;
  progress: number;
  selection: ExportSelection;
  state: JobState;
  updatedAt: string;
  version: number;
  workspaceId: string;
}

export interface JobDetails {
  contractVersion: number;
  job: Job;
}
export interface Job {
  artifactBytes?: number | null;
  artifactHash?: string | null;
  artifactId?: string | null;
  checkpoint: string;
  checkpointVersion: number;
  createdAt: string;
  id: string;
  log: string;
  maxBytes: number;
  maxRecords: number;
  progress: number;
  selection: ExportSelection;
  state: JobState;
  updatedAt: string;
  version: number;
  workspaceId: string;
}

export interface JobList {
  contractVersion: number;
  jobs: Job[];
  nextOffset?: number | null;
}
export interface Job {
  artifactBytes?: number | null;
  artifactHash?: string | null;
  artifactId?: string | null;
  checkpoint: string;
  checkpointVersion: number;
  createdAt: string;
  id: string;
  log: string;
  maxBytes: number;
  maxRecords: number;
  progress: number;
  selection: ExportSelection;
  state: JobState;
  updatedAt: string;
  version: number;
  workspaceId: string;
}
export interface ListJobsRequest {
  contractVersion: number;
  offset?: number;
  workspaceId: string;
}
export interface ListRelationshipsRequest {
  contractVersion: number;
  includeHistory?: boolean;
  predicate?: string | null;
  recordId?: string | null;
  workspaceId: string;
}
export type MemoryState = "proposed" | "confirmed" | "temporary" | "durable" | "superseded" | "archived";
export type Privacy = "sealed" | "private" | "standard" | "public";

export interface MemoryDetails {
  body: string;
  contractVersion: number;
  memory: MemoryMetadata;
}
export interface MemoryMetadata {
  archivedState?: MemoryState | null;
  completed?: boolean | null;
  confidence?: number | null;
  createdAt: string;
  dueDate?: string | null;
  duplicateOf?: string | null;
  eventDate?: string | null;
  generatorModel?: string | null;
  id: string;
  kind: string;
  lineNumber?: number | null;
  privacy: Privacy;
  projectId?: string | null;
  provenance: string;
  relationships: string[];
  restoredFrom?: string | null;
  revisionId: string;
  searchScore?: number | null;
  sectionHeading?: string | null;
  snippet?: string | null;
  sourceReferences: string[];
  status: MemoryState;
  supersededBy?: string | null;
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
  completed?: boolean | null;
  confidence?: number | null;
  dueDate?: string | null;
  eventDate?: string | null;
  privacy?: "sealed" | "private" | "standard" | "public";
  projectId?: string | null;
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
  archivedState?: MemoryState | null;
  completed?: boolean | null;
  confidence?: number | null;
  createdAt: string;
  dueDate?: string | null;
  duplicateOf?: string | null;
  eventDate?: string | null;
  generatorModel?: string | null;
  id: string;
  kind: string;
  lineNumber?: number | null;
  privacy: Privacy;
  projectId?: string | null;
  provenance: string;
  relationships: string[];
  restoredFrom?: string | null;
  revisionId: string;
  searchScore?: number | null;
  sectionHeading?: string | null;
  snippet?: string | null;
  sourceReferences: string[];
  status: MemoryState;
  supersededBy?: string | null;
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
  fullBody?: boolean;
  id?: string | null;
  includeArchived?: boolean;
  includeExpired?: boolean;
  kind?: string | null;
  limit?: number | null;
  offset?: number;
  query?: string | null;
  workspaceId: string;
}

export interface MemoryMetadata {
  archivedState?: MemoryState | null;
  completed?: boolean | null;
  confidence?: number | null;
  createdAt: string;
  dueDate?: string | null;
  duplicateOf?: string | null;
  eventDate?: string | null;
  generatorModel?: string | null;
  id: string;
  kind: string;
  lineNumber?: number | null;
  privacy: Privacy;
  projectId?: string | null;
  provenance: string;
  relationships: string[];
  restoredFrom?: string | null;
  revisionId: string;
  searchScore?: number | null;
  sectionHeading?: string | null;
  snippet?: string | null;
  sourceReferences: string[];
  status: MemoryState;
  supersededBy?: string | null;
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
  kind?: string | null;
  lifecycle?: LifecycleChange | null;
  restoreArchive?: boolean;
  restoreRevision?: string | null;
  workspaceId: string;
}
export interface MemoryDraft {
  body: string;
  completed?: boolean | null;
  confidence?: number | null;
  dueDate?: string | null;
  eventDate?: string | null;
  privacy?: "sealed" | "private" | "standard" | "public";
  projectId?: string | null;
  sourceReferences?: string[];
  title: string;
  validFrom?: string | null;
  validUntil?: string | null;
}
export interface LifecycleChange {
  /**
   * Mandatory explicit user consent, including confirmation and retention.
   */
  confirmed: boolean;
  state: MemoryState;
  successorId?: string | null;
}

export interface ModelRecommendation {
  description: string;
  fitLevel: FitLevel;
  minimumRamBytes: number;
  modelTag: string;
  parameterSize: string;
  quantization: string;
}
export interface OpenWorkspaceRequest {
  contractVersion: number;
  id: string;
}
export interface ReadArtifactRequest {
  artifactId: string;
  contractVersion: number;
  jobId: string;
  offset: number;
  workspaceId: string;
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
export interface RelationshipDetails {
  contractVersion: number;
  relationship: RelationshipRecord;
}
export interface RelationshipRecord {
  archived: boolean;
  createdAt: string;
  id: string;
  metadata?: {
    [k: string]: unknown;
  };
  predicate: string;
  provenance: string;
  sourceId: string;
  targetId: string;
  updatedAt: string;
  version: number;
  workspaceId: string;
}
export interface RelationshipDraft {
  metadata?: {
    [k: string]: unknown;
  };
  predicate: string;
  sourceId: string;
  targetId: string;
}
export interface RelationshipList {
  contractVersion: number;
  relationships: RelationshipRecord[];
}
export interface RelationshipRecord {
  archived: boolean;
  createdAt: string;
  id: string;
  metadata?: {
    [k: string]: unknown;
  };
  predicate: string;
  provenance: string;
  sourceId: string;
  targetId: string;
  updatedAt: string;
  version: number;
  workspaceId: string;
}
export interface RelationshipRecord {
  archived: boolean;
  createdAt: string;
  id: string;
  metadata?: {
    [k: string]: unknown;
  };
  predicate: string;
  provenance: string;
  sourceId: string;
  targetId: string;
  updatedAt: string;
  version: number;
  workspaceId: string;
}
export interface RestoreRequest {
  artifactId: string;
  confirm: boolean;
  contractVersion: number;
  expectedHash: string;
  jobId: string;
  name: string;
  workspaceId: string;
}
export interface RestoreResult {
  backupHash: string;
  contractVersion: number;
  workspaces: Workspace[];
}
export interface Workspace {
  id: string;
  name: string;
}
export interface ScheduleReadRequest {
  contractVersion: number;
  workspaceId: string;
}
export interface ScheduleRequest {
  contractVersion: number;
  enabled: boolean;
  intervalSeconds: number;
  keepAllVersions: boolean;
  managedDestination: boolean;
  plaintextConsent: boolean;
  workspaceId: string;
}
export interface SnapshotPreview {
  application: boolean;
  conflicts: string[];
  contractVersion: number;
  privacy: string;
  records: number;
  relationships: number;
  revisions: number;
  schemaVersion: number;
  sha256: string;
  totalBytes: number;
  workspaceCount: number;
  workspaceId: string;
  workspaceName: string;
}
export interface SnapshotRequest {
  artifactId: string;
  contractVersion: number;
  jobId: string;
  workspaceId: string;
}
export type Intent = "second_memory" | "invent_product" | "build_application" | "connect_ai" | "explore";

export interface UpdateSettingsRequest {
  contractVersion: number;
  id: string;
  intent?: Intent | null;
  onboardingComplete?: boolean | null;
  reducedMotion: boolean;
}
export interface UploadBegin {
  bytes: number;
  contractVersion: number;
  plaintextConsent: boolean;
  sha256: string;
  workspaceId: string;
}
export interface UploadPart {
  bytes: number[];
  contractVersion: number;
  finish: boolean;
  id: string;
  offset: number;
  workspaceId: string;
}

export interface UploadStatus {
  contractVersion: number;
  id: string;
  job?: Job | null;
  received: number;
}
export interface Job {
  artifactBytes?: number | null;
  artifactHash?: string | null;
  artifactId?: string | null;
  checkpoint: string;
  checkpointVersion: number;
  createdAt: string;
  id: string;
  log: string;
  maxBytes: number;
  maxRecords: number;
  progress: number;
  selection: ExportSelection;
  state: JobState;
  updatedAt: string;
  version: number;
  workspaceId: string;
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
