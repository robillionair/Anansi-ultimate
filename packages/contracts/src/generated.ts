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
  embeddingModel?: string | null;
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
export interface AskRunRequest {
  contractVersion: number;
  id: string;
  workspaceId: string;
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

export interface AskRunResponse {
  contractVersion: number;
  error?: CommandError | null;
  id: string;
  partialAnswer: string;
  response?: AskResponse | null;
  state: string;
}
export interface CommandError {
  code: ErrorCode;
  message: string;
}
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

export interface CommandError {
  code: ErrorCode;
  message: string;
}
export interface CompiledContext {
  citations: GroundedCitation[];
  contractVersion: number;
  disclosureManifest: DisclosureManifest;
  evidenceChunks: ContextChunk[];
  fallbackReason?: string | null;
  lexicalOnly?: boolean;
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
  embeddingModel?: string | null;
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
  endpoint?: string | null;
  includeArchived?: boolean;
  includeExpired?: boolean;
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
  embeddingModel?: string | null;
  fallbackReason?: string | null;
  lexicalOnly?: boolean;
  results: HybridSearchResult[];
}
export interface HybridSearchResult {
  cosineSimilarity?: number | null;
  kind: string;
  lexicalRank?: number | null;
  lineNumber?: number | null;
  memoryId: string;
  rerankProvenance?: string | null;
  rerankScore?: number | null;
  rrfScore: number;
  sectionHeading?: string | null;
  semanticRank?: number | null;
  snippet?: string | null;
  title: string;
}
export interface HybridSearchResult {
  cosineSimilarity?: number | null;
  kind: string;
  lexicalRank?: number | null;
  lineNumber?: number | null;
  memoryId: string;
  rerankProvenance?: string | null;
  rerankScore?: number | null;
  rrfScore: number;
  sectionHeading?: string | null;
  semanticRank?: number | null;
  snippet?: string | null;
  title: string;
}
export interface IndexEmbeddingsRequest {
  contractVersion: number;
  endpoint?: string | null;
  memoryId?: string | null;
  model?: string | null;
  workspaceId: string;
}
export interface IndexEmbeddingsResponse {
  contractVersion: number;
  dimension: number;
  indexedChunks: number;
  indexedMemories: number;
  model: string;
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
export interface ProviderAuthorizeRequest {
  allowUnknownCost?: boolean;
  maxEstimatedTotalTokens: number;
  model: string;
  provider: string;
  ttlSeconds?: number | null;
  workspaceId: string;
}
export interface ProviderAuthorizeResponse {
  expiresAtMs: number;
  permitId: string;
}
/**
 * Project budget profiles (§67). The named profiles are qualitative in
 * the spec; ceilings are always caller-supplied numbers, never
 * invented allowances (§16). `FreeOnly` is the "Max Free mode" profile
 * (§16; decision M5-D7) and is meant to be combined with the lock.
 */
export type BudgetProfile = "freeOnly" | "lowCost" | "balanced" | "maximumQuality" | "custom";
/**
 * Where a provider target runs, for the configuration format.
 */
export type LocalityConfig = "local" | "cloud";
/**
 * Cost knowledge about a model. `Unknown` is treated as paid everywhere
 * cost is enforced (decision M5-D5).
 */
export type CostClass = "free" | "paid" | "unknown";
export type SpeedTier = "fast" | "balanced" | "deep";

export interface ProviderConfigureRequest {
  config: RuntimeConfig;
  /**
   * Logical workspace (validated by `Core` against the storage
   * registry before this DTO is ever parsed).
   */
  workspaceId: string;
}
/**
 * The complete provider configuration an application persists and
 * hands to the runtime. Credentials are NOT part of it — only
 * `ProviderConfig.credential` references, resolved at dispatch time
 * through the secret resolver (§15: configuration and secrets are
 * separate stores).
 */
export interface RuntimeConfig {
  budget?: Budget;
  providers: RuntimeProvider[];
}
/**
 * The dispatch budget for this configuration (requests, tokens,
 * estimated cost). Wired into every dispatch — there is no
 * implicit unlimited default once a budget is configured.
 */
export interface Budget {
  /**
   * Micro-USD ceiling, enforced only when a price is configured for
   * the model (prices are caller-supplied with provenance, M5-D5).
   */
  maxEstimatedCostMicroUsd?: number | null;
  maxRequests?: number | null;
  maxTotalTokens?: number | null;
  profile?: BudgetProfile | null;
}
/**
 * One configured provider: endpoint/credential configuration, its
 * locality, and the models the user has configured for it.
 */
export interface RuntimeProvider {
  config: ProviderConfig;
  locality: LocalityConfig;
  models: ModelInfo[];
}
export interface ProviderConfig {
  /**
   * Explicit owner opt-in for a plaintext non-loopback custom
   * endpoint (e.g. an internal enterprise gateway). Never valid
   * together with a credential.
   */
  allowInsecureCustom?: boolean;
  /**
   * Base URL as entered by the user / supplied by the catalog.
   */
  baseUrl: string;
  /**
   * Credential reference only; never a secret value (§15, §70).
   */
  credential?: CredentialRef | null;
  /**
   * True for user-defined OpenAI-compatible / enterprise endpoints
   * (§14). Custom endpoints get the same policy checks, not fewer.
   */
  customEndpoint?: boolean;
  enabled?: boolean;
  provider: string;
}
/**
 * A reference to a credential held by the native secret boundary.
 * Contains no secret material; safe to persist and to log.
 */
export interface CredentialRef {
  /**
   * Provider this credential belongs to, e.g. `openai`.
   */
  provider: string;
  /**
   * Opaque vault account/reference identifier chosen by the user at
   * setup time (MASTER_SPEC §15: the user explicitly pastes the key
   * into the OS vault; ANANSI stores only this reference).
   */
  reference: string;
}
export interface ModelInfo {
  capabilities: CapabilitySet;
  contextWindow?: number | null;
  cost: CostClass;
  id: string;
  /**
   * Caller-supplied price (micro-USD per 1M tokens) with provenance in
   * configuration. The crate ships no price table (M5-D5).
   */
  priceInputMicroUsdPerMtok?: number | null;
  priceOutputMicroUsdPerMtok?: number | null;
  provider: string;
  speed: SpeedTier;
}
/**
 * Capability flags a model can be routed on (§17: capability is one of
 * the router's evaluation inputs).
 */
export interface CapabilitySet {
  chat: boolean;
  coding: boolean;
  deepReasoning: boolean;
  embeddings: boolean;
  streaming: boolean;
  vision: boolean;
}
export interface ProviderConfigureResponse {
  providers: string[];
}
export type Role = "system" | "user" | "assistant";
/**
 * Privacy classification of one content segment. Mirrors the classes in
 * `anansi-core` (`crates/core/src/privacy.rs`) plus `Restricted`, which
 * MASTER_SPEC §19 uses without defining; see decision M5-D4 in
 * `docs/handoffs/M5-requirements-and-acceptance.md`.
 */
export type PrivacyClass = "public" | "standard" | "private" | "sealed" | "restricted";

/**
 * Dispatch and stream-start share one request shape. It carries
 * **no** configuration, locality, cost or credential material:
 * policy is resolved from backend state only.
 */
export interface ProviderDispatchRequest {
  /**
   * @minItems 2
   * @maxItems 2
   */
  chosen?: [unknown, unknown] | null;
  mode?: string | null;
  /**
   * The id of a permit issued by `providers-authorize` to this
   * session. The permit itself (scope, caps, expiry) lives in
   * backend state; presenting an id never creates authorization.
   */
  permitId?: string | null;
  request: CompletionRequest;
  workspaceId: string;
  zeroCostLock?: boolean;
}
export interface CompletionRequest {
  /**
   * Explicit user authorization to send `Private` segments to a cloud
   * provider (ADR-0002: private material requires explicit
   * authorization). Never set implicitly by routing.
   */
  explicitCloudAuthorization?: boolean;
  /**
   * The caller asserts this content must never leave the local machine
   * (compiled from local-only memories). The router and gates enforce
   * it; it is never silently dropped (§17).
   */
  localOnly?: boolean;
  maxOutputTokens?: number | null;
  messages: ChatMessage[];
  model: string;
  temperature?: number | null;
}
export interface ChatMessage {
  role: Role;
  segments: Segment[];
}
/**
 * One classified piece of a message. `redacted` means the caller has
 * already replaced the sensitive content of `text`; the dispatcher
 * refuses unredacted `Restricted` segments for cloud dispatch and never
 * guesses at redaction itself.
 */
export interface Segment {
  privacy: PrivacyClass;
  redacted?: boolean;
  /**
   * Source memory/record identifier, preserved for citations (§19).
   */
  sourceId?: string | null;
  text: string;
}
/**
 * Cost knowledge about a model. `Unknown` is treated as paid everywhere
 * cost is enforced (decision M5-D5).
 */
/**
 * User-facing routing modes, verbatim from MASTER_SPEC §17.
 */
export type RoutingMode = "auto" | "autoFree" | "private" | "fast" | "deep" | "coding" | "vision" | "chooseModel";

export interface ProviderDispatchResponse {
  disclosure: DisclosureRecord;
  response: ProviderResponseBody;
}
/**
 * "View What ANANSI Sent" at adapter level (§19): what was about to be
 * / was transmitted, to whom, and why. The UI rendering of this record
 * is application integration; producing it is the dispatcher's job.
 */
export interface DisclosureRecord {
  attempts: AttemptRecord[];
  costClass: CostClass;
  endpointHost: string;
  estimatedInputTokens: number;
  local: boolean;
  messageCount: number;
  model: string;
  provider: string;
  redactionCount: number;
  routingMode: RoutingMode;
  routingReason: string;
  segmentCount: number;
  usage?: Usage | null;
}
export interface AttemptRecord {
  model: string;
  outcome: string;
  provider: string;
}
export interface Usage {
  inputTokens: number;
  outputTokens: number;
}
export interface ProviderResponseBody {
  finishReason?: string | null;
  model: string;
  text: string;
  usage?: Usage | null;
}
export interface ProviderGrantRequest {
  /**
   * The session label to grant (for API sessions, the token
   * fingerprint the API stamps — the host knows it from the
   * pairing record; the label is not a secret).
   */
  session: string;
  ttlSeconds?: number | null;
  workspaceId: string;
}
/**
 * Cost knowledge about a model. `Unknown` is treated as paid everywhere
 * cost is enforced (decision M5-D5).
 */
/**
 * User-facing routing modes, verbatim from MASTER_SPEC §17.
 */

export interface ProviderPollResponse {
  disclosure: DisclosureRecord;
  done: boolean;
  events: ProviderStreamEvent[];
}
/**
 * "View What ANANSI Sent" at adapter level (§19): what was about to be
 * / was transmitted, to whom, and why. The UI rendering of this record
 * is application integration; producing it is the dispatcher's job.
 */
export interface DisclosureRecord {
  attempts: AttemptRecord[];
  costClass: CostClass;
  endpointHost: string;
  estimatedInputTokens: number;
  local: boolean;
  messageCount: number;
  model: string;
  provider: string;
  redactionCount: number;
  routingMode: RoutingMode;
  routingReason: string;
  segmentCount: number;
  usage?: Usage | null;
}
export interface AttemptRecord {
  model: string;
  outcome: string;
  provider: string;
}
export interface Usage {
  inputTokens: number;
  outputTokens: number;
}
export interface ProviderStreamEvent {
  code?: string | null;
  finishReason?: string | null;
  inputTokens?: number | null;
  kind: string;
  message?: string | null;
  outputTokens?: number | null;
  text?: string | null;
}
export interface ProviderRunRequest {
  capability: string;
  runId: string;
  workspaceId: string;
}
/**
 * Cost knowledge about a model. `Unknown` is treated as paid everywhere
 * cost is enforced (decision M5-D5).
 */
/**
 * User-facing routing modes, verbatim from MASTER_SPEC §17.
 */

export interface ProviderStreamStartResponse {
  /**
   * Unpredictable per-run capability; poll/cancel require it.
   */
  capability: string;
  disclosure: DisclosureRecord;
  runId: string;
}
/**
 * "View What ANANSI Sent" at adapter level (§19): what was about to be
 * / was transmitted, to whom, and why. The UI rendering of this record
 * is application integration; producing it is the dispatcher's job.
 */
export interface DisclosureRecord {
  attempts: AttemptRecord[];
  costClass: CostClass;
  endpointHost: string;
  estimatedInputTokens: number;
  local: boolean;
  messageCount: number;
  model: string;
  provider: string;
  redactionCount: number;
  routingMode: RoutingMode;
  routingReason: string;
  segmentCount: number;
  usage?: Usage | null;
}
export interface AttemptRecord {
  model: string;
  outcome: string;
  provider: string;
}
export interface Usage {
  inputTokens: number;
  outputTokens: number;
}
export interface ProviderUsageResponse {
  providers: ProviderUsageEntry[];
  reservedRequests: number;
  reservedTokens: number;
  totalRequests: number;
}
export interface ProviderUsageEntry {
  inputTokens: number;
  outputTokens: number;
  provider: string;
  requests: number;
}
export interface ProviderWorkspaceRequest {
  workspaceId: string;
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
