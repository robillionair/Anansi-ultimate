import { validReadiness, validError, validWorkspaceDetails, validWorkspaceList, validMemoryDetails, validMemoryList, validRelationshipDetails, validRelationshipList, validJobDetails, validJobList, validArtifactChunk, validAiDiscoveryResponse, validAskResponse, validHybridSearchResponse, validCompiledContext, validIndexEmbeddingsResponse } from './validators.mjs';
import { invoke } from '@tauri-apps/api/core';
import { validAskRunResponse, validProviderCatalogResponse, validProviderConfigureResponse, validProviderAuthorizeResponse, validProviderDispatchResponse, validProviderStreamStartResponse, validProviderPollResponse, validProviderUsageResponse, validProviderCredentialSetResponse, validProviderCredentialRemoveResponse, validProviderCredentialStatusResponse, validProviderSessionAccessResponse, validArtifactDetails, validArtifactListResponse, validArtifactVersionsResponse, validIdeasGenerateStartResponse, validIdeasPollResponse, validIdeasSaveResponse, validOpportunitiesGenerateStartResponse, validOpportunitiesPollResponse, validOpportunitiesSaveResponse, validEvaluationsSaveResponse, validStandardRubricResponse, validForgeStartResponse, validForgePollResponse, validForgeSaveResponse, validForgeExportResponse, validForgeSpecApproveResponse, validForgeSpecApprovalStatusResponse } from './validators.mjs';
import type { EvaluationsSaveRequest, EvaluationsSaveResponse, RubricDoc, StandardRubricResponse, IdeasGenerateStartRequest, IdeasGenerateStartResponse, IdeasPollResponse, IdeasSaveResponse, IdeaSet, IdeaConcept, OpportunitiesGenerateStartRequest, OpportunitiesGenerateStartResponse, OpportunitiesPollResponse, OpportunitiesSaveResponse, OpportunitySet, OpportunityConcept, OpportunitySourceRecord, SuppliedSourceInput, ForgeSpecStartRequest, ForgeBuildStartRequest, ForgeStartResponse, ForgePollResponse, ForgeSaveResponse, ForgeExportResponse, ForgeSpecApproveRequest, ForgeSpecApproveResponse, ForgeSpecApprovalStatusRequest, ForgeSpecApprovalStatusResponse, SpecificationApprovalDoc, ProductSpecDoc, SourcePackageDoc, AskRunResponse, ProviderCatalogResponse, ProviderConfigureResponse, ProviderAuthorizeResponse, ProviderDispatchResponse, ProviderStreamStartResponse, ProviderPollResponse, ProviderUsageResponse, RuntimeConfig, CompletionRequest, ProviderCredentialSetResponse, ProviderCredentialRemoveResponse, ProviderCredentialStatusResponse, ProviderSessionAccessResponse } from './generated';
export type { EvaluationsSaveRequest, EvaluationsSaveResponse, RubricDoc, RubricDimension, AssessmentInput, ConceptSummary, StandardRubricResponse, IdeasGenerateStartRequest, IdeasGenerateStartResponse, IdeasPollResponse, IdeasSaveResponse, IdeaSet, IdeaConcept, OpportunitiesGenerateStartRequest, OpportunitiesGenerateStartResponse, OpportunitiesPollResponse, OpportunitiesSaveResponse, OpportunitySet, OpportunityConcept, OpportunitySourceRecord, SuppliedSourceInput, ForgeSpecStartRequest, ForgeBuildStartRequest, ForgeRunRequest, ForgeStartResponse, ForgePollResponse, ForgeSaveResponse, ForgeExportRequest, ForgeExportResponse, ForgeSpecApproveRequest, ForgeSpecApproveResponse, ForgeSpecApprovalStatusRequest, ForgeSpecApprovalStatusResponse, SpecificationApprovalDoc, ProductSpec, ProductSpecDoc, ForgeAnswers, DevelopmentPlan, SourceFile, SourcePackageDoc, AskRunResponse, ProviderCatalogResponse, ProviderCatalogEntry, ProviderConfigureResponse, ProviderAuthorizeResponse, ProviderDispatchResponse, ProviderStreamStartResponse, ProviderPollResponse, ProviderUsageResponse, ProviderUsageEntry, RuntimeConfig, CompletionRequest, DisclosureRecord, ProviderCredentialSetResponse, ProviderCredentialRemoveResponse, ProviderCredentialStatusResponse, ProviderSessionAccessResponse, ProviderPendingAccess, ProviderGrantInfo } from './generated';
import {validSnapshotPreview,validRestoreResult,validBackupSchedule,validUploadStatus} from './validators.mjs';
import type {SnapshotPreview,RestoreResult,BackupSchedule,AiDiscoveryResponse,HardwareProfile,DiscoveredRuntime,ModelRecommendation,RuntimeKind,RuntimeStatus,FitLevel,AskRequest,AskResponse,AskStatus,GroundedCitation,DisclosureManifest,HybridSearchRequest,HybridSearchResult,HybridSearchResponse,ContextCompilationRequest,CompiledContext,ContextChunk,IndexEmbeddingsRequest,IndexEmbeddingsResponse} from './generated';
export type {SnapshotPreview,RestoreResult,BackupSchedule,AiDiscoveryResponse,HardwareProfile,DiscoveredRuntime,ModelRecommendation,RuntimeKind,RuntimeStatus,FitLevel,AskRequest,AskResponse,AskStatus,GroundedCitation,DisclosureManifest,HybridSearchRequest,HybridSearchResult,HybridSearchResponse,ContextCompilationRequest,CompiledContext,ContextChunk,IndexEmbeddingsRequest,IndexEmbeddingsResponse} from './generated';
import type { ArtifactDetails, ArtifactListResponse, ArtifactVersionsResponse, ArtifactRecord } from './generated';
export type { ArtifactDetails, ArtifactListResponse, ArtifactVersionsResponse, ArtifactRecord } from './generated';
import type { Readiness, WorkspaceDetails, WorkspaceList, Intent, MemoryDraft, MemoryDetails, MemoryList, MemoryMetadata, Privacy, LifecycleChange, RelationshipDraft, RelationshipRecord, RelationshipDetails, RelationshipList, Job, JobDetails, JobList, ArtifactChunk, ExportSelection, JobAction } from './generated';
export type { Readiness, WorkspaceDetails, WorkspaceList, Intent, MemoryDraft, MemoryDetails, MemoryList, MemoryMetadata, Privacy, LifecycleChange, RelationshipDraft, RelationshipRecord, RelationshipDetails, RelationshipList, Job, JobDetails, JobList, ArtifactChunk, ExportSelection, JobAction } from './generated';
export type VaultCommandName = 'setup'|'seal'|'unlock'|'read'|'lock'|'rotate'|'backup'|'export'|'preview'|'restore';
export type EvaluationsSaveInput = Omit<EvaluationsSaveRequest, 'contractVersion' | 'workspaceId'>;
export type IdeasStartInput = Omit<IdeasGenerateStartRequest, 'contractVersion' | 'workspaceId'>;
export type OpportunitiesStartInput = Omit<OpportunitiesGenerateStartRequest, 'contractVersion' | 'workspaceId'>;
export type ForgeSpecStartInput = Omit<ForgeSpecStartRequest, 'contractVersion' | 'workspaceId'>;
export type ForgeBuildStartInput = Omit<ForgeBuildStartRequest, 'contractVersion' | 'workspaceId'>;
export type ForgeSpecApproveInput = Omit<ForgeSpecApproveRequest, 'contractVersion' | 'workspaceId'>;
export type ForgeSpecApprovalStatusInput = Omit<ForgeSpecApprovalStatusRequest, 'contractVersion' | 'workspaceId'>;
export interface Client {
  vaultCommand?:(command:VaultCommandName,request:Record<string,unknown>)=>Promise<Record<string,unknown>>;
  startAsk(workspaceId:string,prompt:string,options?:{runtimeEndpoint?:string;modelTag?:string;embeddingModel?:string}):Promise<AskRunResponse>;
  pollAsk(workspaceId:string,id:string):Promise<AskRunResponse>;
  cancelAsk(workspaceId:string,id:string):Promise<AskRunResponse>;
  forgetAsk(workspaceId:string,id:string):Promise<AskRunResponse>;
  importBackup(workspaceId:string,bytes:Uint8Array,plaintextConsent:boolean):Promise<Job>;
  enqueueBackup(workspaceId:string,application:boolean,plaintextConsent:boolean):Promise<JobDetails>;
  previewBackup(workspaceId:string,jobId:string,artifactId:string):Promise<SnapshotPreview>;
  restoreBackup(workspaceId:string,jobId:string,artifactId:string,name:string,expectedHash:string,confirm:boolean):Promise<RestoreResult>;
  readBackupSchedule(workspaceId:string):Promise<BackupSchedule>;
  setBackupSchedule(workspaceId:string,enabled:boolean,intervalSeconds:number,plaintextConsent:boolean,keepAllVersions:boolean,managedDestination:boolean):Promise<BackupSchedule>;
  enqueueExport(workspaceId:string,selection:ExportSelection):Promise<JobDetails>;
  listJobs(workspaceId:string,offset?:number):Promise<JobList>;
  controlJob(workspaceId:string,id:string,expectedVersion:number,action:JobAction):Promise<JobDetails>;
  readArtifact(workspaceId:string,jobId:string,artifactId:string,offset:number):Promise<ArtifactChunk>;
  listMemories(workspaceId:string, options?:{id?:string;offset?:number;limit?:number;kind?:string;query?:string;includeArchived?:boolean;includeExpired?:boolean;fullBody?:boolean}): Promise<MemoryList>;
  readMemory(workspaceId:string,id:string,revisionId?:string): Promise<MemoryDetails>;
  createMemory(workspaceId:string,draft:MemoryDraft,kind?:string): Promise<MemoryDetails>;
  reviseMemory(workspaceId:string,id:string,expectedRevision:string,draft:MemoryDraft,confirmPrivacyChange?:boolean,options?:{lifecycle?:LifecycleChange;restoreArchive?:boolean;restoreRevision?:string}): Promise<MemoryDetails>;
  createRelationship(workspaceId:string,draft:RelationshipDraft): Promise<RelationshipDetails>;
  deleteRelationship(workspaceId:string,id:string,expectedVersion:number): Promise<RelationshipDetails>;
  restoreRelationship(workspaceId:string,id:string,expectedVersion:number): Promise<RelationshipDetails>;
  listRelationships(workspaceId:string,options?:{recordId?:string;predicate?:string;includeHistory?:boolean}): Promise<RelationshipList>;
  readiness(): Promise<Readiness>;
  listWorkspaces(): Promise<WorkspaceList>;
  createWorkspace(name: string): Promise<WorkspaceDetails>;
  openWorkspace(id: string): Promise<WorkspaceDetails>;
  updateSettings(id: string, reducedMotion: boolean, preferences?: {intent?:Intent;onboardingComplete?:boolean}): Promise<WorkspaceDetails>;
  aiDiscovery(customEndpoints?: string[]): Promise<AiDiscoveryResponse>;
  askAnansi(workspaceId: string, prompt: string, options?: { runtimeEndpoint?: string; modelTag?: string; embeddingModel?: string; maxEvidenceItems?: number }): Promise<AskResponse>;
  hybridSearch(workspaceId: string, options?: { query?: string; model?: string; endpoint?: string; limit?: number; lexicalWeight?: number; semanticWeight?: number; rrfK?: number; kind?: string; includeArchived?: boolean; includeExpired?: boolean }): Promise<HybridSearchResponse>;
  compileContext(workspaceId: string, userPrompt: string, options?: { maxBudgetChars?: number; runtimeEndpoint?: string; modelTag?: string; embeddingModel?: string; kindFilter?: string }): Promise<CompiledContext>;
  indexEmbeddings(workspaceId: string, options?: { memoryId?: string; model?: string; endpoint?: string }): Promise<IndexEmbeddingsResponse>;
  providersCatalog(): Promise<ProviderCatalogResponse>;
  providersConfigure(workspaceId: string, config: RuntimeConfig): Promise<ProviderConfigureResponse>;
  providersAuthorize(workspaceId: string, provider: string, model: string, maxEstimatedTotalTokens: number, allowUnknownCost?: boolean): Promise<ProviderAuthorizeResponse>;
  providersGrantSession(workspaceId: string, session: string, ttlSeconds?: number): Promise<{ granted: boolean }>;
  providersRevokeSession(workspaceId: string, session: string): Promise<{ granted: boolean }>;
  providersDispatch(workspaceId: string, request: CompletionRequest, options?: { mode?: string; zeroCostLock?: boolean; chosen?: [string, string]; permitId?: string }): Promise<ProviderDispatchResponse>;
  providersStreamStart(workspaceId: string, request: CompletionRequest, options?: { mode?: string; zeroCostLock?: boolean; chosen?: [string, string]; permitId?: string }): Promise<ProviderStreamStartResponse>;
  providersStreamPoll(workspaceId: string, runId: string, capability: string): Promise<ProviderPollResponse>;
  providersStreamCancel(workspaceId: string, runId: string, capability: string): Promise<{ cancelled: boolean }>;
  providersUsage(workspaceId: string): Promise<ProviderUsageResponse>;
  registerProjectArtifact(workspaceId: string, projectId: string, artifact: { type: string; title: string; body: string; sourceReferences?: string[]; privacy?: string }): Promise<ArtifactDetails>;
  reviseProjectArtifact(workspaceId: string, artifactId: string, expectedVersion: number, artifact: { title: string; body: string; sourceReferences?: string[] }): Promise<ArtifactDetails>;
  listProjectArtifacts(workspaceId: string, projectId: string): Promise<ArtifactListResponse>;
  listProjectArtifactVersions(workspaceId: string, artifactId: string): Promise<ArtifactVersionsResponse>;
  readProjectArtifact(workspaceId: string, artifactId: string, version?: number): Promise<ArtifactDetails>;
  providersCredentialSet(workspaceId: string, provider: string, reference: string): Promise<ProviderCredentialSetResponse>;
  providersCredentialRemove(workspaceId: string, provider: string, reference: string): Promise<ProviderCredentialRemoveResponse>;
  providersCredentialStatus(workspaceId: string, provider: string, reference: string): Promise<ProviderCredentialStatusResponse>;
  providersSessionAccess(workspaceId: string): Promise<ProviderSessionAccessResponse>;
  startProjectIdeas(workspaceId: string, input: IdeasStartInput): Promise<IdeasGenerateStartResponse>;
  pollProjectIdeas(workspaceId: string, runId: string): Promise<IdeasPollResponse>;
  cancelProjectIdeas(workspaceId: string, runId: string): Promise<IdeasPollResponse>;
  saveProjectIdeas(workspaceId: string, runId: string): Promise<IdeasSaveResponse>;
  startProjectOpportunities(workspaceId: string, input: OpportunitiesStartInput): Promise<OpportunitiesGenerateStartResponse>;
  pollProjectOpportunities(workspaceId: string, runId: string): Promise<OpportunitiesPollResponse>;
  cancelProjectOpportunities(workspaceId: string, runId: string): Promise<OpportunitiesPollResponse>;
  saveProjectOpportunities(workspaceId: string, runId: string): Promise<OpportunitiesSaveResponse>;
  startForgeSpec(workspaceId: string, input: ForgeSpecStartInput): Promise<ForgeStartResponse>;
  pollForgeSpec(workspaceId: string, runId: string): Promise<ForgePollResponse>;
  cancelForgeSpec(workspaceId: string, runId: string): Promise<ForgePollResponse>;
  saveForgeSpec(workspaceId: string, runId: string): Promise<ForgeSaveResponse>;
  approveForgeSpec(workspaceId: string, input: ForgeSpecApproveInput): Promise<ForgeSpecApproveResponse>;
  specApprovalStatus(workspaceId: string, input: ForgeSpecApprovalStatusInput): Promise<ForgeSpecApprovalStatusResponse>;
  startForgeBuild(workspaceId: string, input: ForgeBuildStartInput): Promise<ForgeStartResponse>;
  pollForgeBuild(workspaceId: string, runId: string): Promise<ForgePollResponse>;
  cancelForgeBuild(workspaceId: string, runId: string): Promise<ForgePollResponse>;
  saveForgeBuild(workspaceId: string, runId: string): Promise<ForgeSaveResponse>;
  exportForgePackage(workspaceId: string, artifactId: string, version: number): Promise<ForgeExportResponse>;
  saveProjectEvaluation(workspaceId: string, input: EvaluationsSaveInput): Promise<EvaluationsSaveResponse>;
  getStandardRubric(workspaceId: string, projectId: string): Promise<StandardRubricResponse>;
}
function decode<T extends {contractVersion:number}>(value:unknown, validate:(value:unknown)=>value is T):T {
  if(!validate(value)||value.contractVersion!==1)throw new Error('The core returned an incompatible response. Update both applications.');
  return value;
}
export function decodeReadiness(value:unknown):Readiness {return decode(value,validReadiness);}
function failure(value:unknown,operation=''):Error {
  if(validError(value)){
    if(operation.startsWith('ai/providers-')){
      if(value.code==='FORBIDDEN'){
        // Two different refusals share the Forbidden code: the
        // owner boundary (configure/authorize without a grant)
        // and the dispatcher's permit gate. The permit refusal is
        // actionable from this screen (authorize the model), so
        // its composed detail is surfaced verbatim.
        if(typeof value.message==='string'&&value.message.includes('PermissionRefused'))return new Error(value.message);
        return new Error('Owner authorization required. Configure and authorize are owner operations: run them in the desktop app, or have the host grant this browser session first.');
      }
      if(value.code==='UNAUTHORIZED')return new Error('The provider refused the stored credential. Check the credential reference in the OS keyring and the provider configuration.');
      // Provider errors carry a composed detail ("Code: message")
      // from the dispatcher — surfacing it is the point of the
      // disclosure workflow (lock refusals, quota, attempts).
      if(typeof value.message==='string'&&value.message)return new Error(value.message);
    }
    if(operation.startsWith('vault/')){
      switch(value.code){
        case 'INVALID_REQUEST':return new Error(operation==='vault/setup'?'Vault setup was not acknowledged. Save the displayed recovery key, then enter that recovery key in the native prompt’s Password field. Do not repeat your vault passphrase. No sealing was authorized by this failed setup.':'The vault request is incomplete or needs a recovery acknowledgement and a verified backup. Check the selected operation and retry.');
        case 'UNAUTHORIZED':return new Error('The vault is locked or the vault passphrase was rejected. Unlock through the native prompt and retry.');
        case 'VAULT_UNAVAILABLE':return new Error('The native vault operation was cancelled, unavailable or could not finish. Retry through the native controls. No plaintext fallback was used.');
        case 'FORBIDDEN':return new Error('This vault request does not match the workspace or application process.');
      }
    }
    if(operation.startsWith('ai/ideas-')||operation.startsWith('ai/opportunities-')){
      // Idea-generation and opportunity-discovery refusals
      // and outcomes are composed for the user at the core
      // (archived project, model not configured, gates) -
      // surface them verbatim.
      if(typeof value.message==='string'&&value.message)return new Error(value.message);
    }
    if(operation.startsWith('artifacts/')){
      // Registry refusals are composed for the user at the core
      // (archived project, stale revision, sealed boundary) and
      // are surfaced verbatim, like provider dispatch detail.
      if(typeof value.message==='string'&&value.message)return new Error(value.message);
    }
    if(operation.startsWith('jobs/')&&value.code==='REVISION_CONFLICT')return new Error('This job changed or its lease expired. Refresh jobs and try again.');
    switch(value.code){
      case 'NOT_FOUND': return new Error('The requested item was not found in this workspace. Try again after refreshing.');
      case 'INVALID_REQUEST': return new Error('Check the selection, resource limits and current job state, then refresh.');
      case 'INVALID_MEMORY': return new Error('Check the fields, lifecycle action, expiry, successor and privacy confirmation. Use a title up to 200 characters, text up to 256 KiB, UTC dates and confidence from 0 to 1.');
      case 'INVALID_SEARCH': return new Error('Use one to eight title words, up to 256 UTF-8 bytes. If the workspace exceeds 10,000 eligible records or 32 MiB of metadata, narrow the record type. History search is unavailable; no partial results were returned.');
      case 'MEMORY_NOT_FOUND': return new Error('Memory not found in this workspace. Refresh the list.');
      case 'REVISION_CONFLICT': return new Error('This memory changed. Your draft was kept. Reopen the latest revision before saving again.');
      case 'CONTENT_INTEGRITY': return new Error('Memory content failed integrity verification. Preserve the storage root and restore a verified copy.');
      case 'VAULT_UNAVAILABLE': return new Error('This operation cannot access Sealed memories. Use the authorized vault workflow. No plaintext fallback was used.');
      case 'UNAUTHORIZED': return new Error('Pairing was rejected. Enter the current session token.');
      case 'FORBIDDEN': return new Error('This browser address is not allowed. Open http://127.0.0.1:1420.');
      case 'UNSUPPORTED_CONTRACT': return new Error('The core uses a different contract version. Update both applications.');
      case 'INVALID_WORKSPACE': return new Error('Use a valid workspace name or choose an existing workspace from the list.');
      case 'WORKSPACE_EXISTS': return new Error('A workspace with that name already exists. Open it from the list or use a different name.');
      case 'WORKSPACE_NOT_FOUND': return new Error('Workspace not found. Refresh the list and choose an existing workspace.');
      case 'STORAGE_UNAVAILABLE': return new Error('Local storage is unavailable. Check the configured folder and its permissions, then retry.');
      case 'UNSUPPORTED_SCHEMA': return new Error('Storage uses an unsupported schema version. Keep the original files and use a compatible ANANSI version.');
      case 'CORRUPT_STORAGE': return new Error('Workspace storage cannot be read. Keep the original files and restore a verified backup.');
    }
  }
  return new Error('The core could not complete the request. Try again.');
}
type Call=(operation:string,request:Record<string,unknown>)=>Promise<unknown>;
function client(call:Call):Client {
  return {
    startAsk:async(workspaceId,prompt,options={})=>decode(await call('ai/ask-start',{workspaceId,prompt,...options}),validAskRunResponse),
    pollAsk:async(workspaceId,id)=>decode(await call('ai/ask-poll',{workspaceId,id}),validAskRunResponse),
    cancelAsk:async(workspaceId,id)=>decode(await call('ai/ask-cancel',{workspaceId,id}),validAskRunResponse),
    forgetAsk:async(workspaceId,id)=>decode(await call('ai/ask-forget',{workspaceId,id}),validAskRunResponse),
    importBackup:async(workspaceId,bytes,plaintextConsent)=>{
      if(bytes.length>256*1024*1024||bytes.length<1024)throw Error('Choose a backup up to 256 MiB.');
      const sha256=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',Uint8Array.from(bytes)))).map(b=>b.toString(16).padStart(2,'0')).join('');
      const opened=decode(await call('jobs/upload-begin',{workspaceId,bytes:bytes.length,sha256,plaintextConsent}),validUploadStatus);
      let result=opened;
      for(let offset=0;offset<bytes.length;offset+=65536){const end=Math.min(offset+65536,bytes.length);result=decode(await call('jobs/upload-part',{workspaceId,id:opened.id,offset,bytes:Array.from(bytes.subarray(offset,end)),finish:end===bytes.length}),validUploadStatus);}
      if(!result.job)throw Error('Backup import did not publish a verified job.');return result.job;
    },
    enqueueBackup:async(workspaceId,application,plaintextConsent)=>decode(await call('jobs/backup',{workspaceId,application,plaintextConsent}),validJobDetails),
    previewBackup:async(workspaceId,jobId,artifactId)=>decode(await call('jobs/preview',{workspaceId,jobId,artifactId}),validSnapshotPreview),
    restoreBackup:async(workspaceId,jobId,artifactId,name,expectedHash,confirm)=>decode(await call('jobs/restore',{workspaceId,jobId,artifactId,name,expectedHash,confirm}),validRestoreResult),
    readBackupSchedule:async(workspaceId)=>decode(await call('jobs/schedule-read',{workspaceId}),validBackupSchedule),
    setBackupSchedule:async(workspaceId,enabled,intervalSeconds,plaintextConsent,keepAllVersions,managedDestination)=>decode(await call('jobs/schedule',{workspaceId,enabled,intervalSeconds,plaintextConsent,keepAllVersions,managedDestination}),validBackupSchedule),
    enqueueExport:async(workspaceId,selection)=>decode(await call('jobs/export',{workspaceId,selection}),validJobDetails),
    listJobs:async(workspaceId,offset=0)=>decode(await call('jobs/list',{workspaceId,offset}),validJobList),
    controlJob:async(workspaceId,id,expectedVersion,action)=>decode(await call('jobs/control',{workspaceId,id,expectedVersion,action}),validJobDetails),
    readArtifact:async(workspaceId,jobId,artifactId,offset)=>decode(await call('jobs/artifact',{workspaceId,jobId,artifactId,offset}),validArtifactChunk),
    listMemories:async(workspaceId,options={})=>decode(await call('memories/list',{workspaceId,...options}),validMemoryList),
    readMemory:async(workspaceId,id,revisionId)=>decode(await call('memories/read',{workspaceId,id,revisionId}),validMemoryDetails),
    createMemory:async(workspaceId,draft,kind)=>decode(await call('memories/create',{workspaceId,draft,kind}),validMemoryDetails),
    reviseMemory:async(workspaceId,id,expectedRevision,draft,confirmPrivacyChange=false,options={})=>decode(await call('memories/revise',{workspaceId,id,expectedRevision,draft,confirmPrivacyChange,...options}),validMemoryDetails),
    createRelationship:async(workspaceId,draft)=>decode(await call('relationships/create',{workspaceId,draft}),validRelationshipDetails),
    deleteRelationship:async(workspaceId,id,expectedVersion)=>decode(await call('relationships/delete',{workspaceId,id,expectedVersion}),validRelationshipDetails),
    restoreRelationship:async(workspaceId,id,expectedVersion)=>decode(await call('relationships/restore',{workspaceId,id,expectedVersion}),validRelationshipDetails),
    listRelationships:async(workspaceId,options={})=>decode(await call('relationships/list',{workspaceId,...options}),validRelationshipList),
    readiness:async()=>decodeReadiness(await call('readiness',{})),
    listWorkspaces:async()=>decode(await call('list',{}),validWorkspaceList),
    createWorkspace:async name=>decode(await call('create',{name}),validWorkspaceDetails),
    openWorkspace:async id=>decode(await call('open',{id}),validWorkspaceDetails),
    updateSettings:async(id,reducedMotion,preferences={})=>decode(await call('settings',{id,reducedMotion,...preferences}),validWorkspaceDetails),
    aiDiscovery:async(customEndpoints)=>decode(await call('ai/discovery',{customEndpoints}),validAiDiscoveryResponse),
    askAnansi:async(workspaceId,prompt,options={})=>decode(await call('ai/ask',{workspaceId,prompt,...options}),validAskResponse),
    hybridSearch:async(workspaceId,options={})=>decode(await call('ai/hybrid-search',{workspaceId,...options}),validHybridSearchResponse),
    compileContext:async(workspaceId,userPrompt,options={})=>decode(await call('ai/context',{workspaceId,userPrompt,...options}),validCompiledContext),
    indexEmbeddings:async(workspaceId,options={})=>decode(await call('ai/index-embeddings',{workspaceId,...options}),validIndexEmbeddingsResponse),
    providersCatalog:async()=>decode(await call('ai/providers-catalog',{}),validProviderCatalogResponse),
    providersConfigure:async(workspaceId,config)=>decode(await call('ai/providers-configure',{workspaceId,config}),validProviderConfigureResponse),
    providersAuthorize:async(workspaceId,provider,model,maxEstimatedTotalTokens,allowUnknownCost)=>decode(await call('ai/providers-authorize',{workspaceId,provider,model,maxEstimatedTotalTokens,allowUnknownCost}),validProviderAuthorizeResponse),
    providersGrantSession:async(workspaceId,session,ttlSeconds)=>{const v=await call('ai/providers-grant-session',{workspaceId,session,ttlSeconds}) as {granted?:unknown};if(typeof v?.granted!=='boolean')throw new Error('The core returned an incompatible response. Update both applications.');return {granted:v.granted};},
    providersRevokeSession:async(workspaceId,session)=>{const v=await call('ai/providers-revoke-session',{workspaceId,session}) as {granted?:unknown};if(typeof v?.granted!=='boolean')throw new Error('The core returned an incompatible response. Update both applications.');return {granted:v.granted};},
    providersDispatch:async(workspaceId,request,options={})=>decode(await call('ai/providers-dispatch',{workspaceId,request,...options}),validProviderDispatchResponse),
    providersStreamStart:async(workspaceId,request,options={})=>decode(await call('ai/providers-stream-start',{workspaceId,request,...options}),validProviderStreamStartResponse),
    providersStreamPoll:async(workspaceId,runId,capability)=>decode(await call('ai/providers-stream-poll',{workspaceId,runId,capability}),validProviderPollResponse),
    providersStreamCancel:async(workspaceId,runId,capability)=>{const v=await call('ai/providers-stream-cancel',{workspaceId,runId,capability}) as {cancelled?:unknown};if(typeof v?.cancelled!=='boolean')throw new Error('The core returned an incompatible response. Update both applications.');return {cancelled:v.cancelled};},
    providersUsage:async(workspaceId)=>decode(await call('ai/providers-usage',{workspaceId}),validProviderUsageResponse),
    registerProjectArtifact:async(workspaceId,projectId,artifact)=>decode(await call('artifacts/register',{workspaceId,projectId,...artifact}),validArtifactDetails),
    reviseProjectArtifact:async(workspaceId,artifactId,expectedVersion,artifact)=>decode(await call('artifacts/revise',{workspaceId,artifactId,expectedVersion,...artifact}),validArtifactDetails),
    listProjectArtifacts:async(workspaceId,projectId)=>decode(await call('artifacts/list',{workspaceId,projectId}),validArtifactListResponse),
    listProjectArtifactVersions:async(workspaceId,artifactId)=>decode(await call('artifacts/versions',{workspaceId,artifactId}),validArtifactVersionsResponse),
    readProjectArtifact:async(workspaceId,artifactId,version)=>decode(await call('artifacts/read',{workspaceId,artifactId,version}),validArtifactDetails),
    providersCredentialSet:async(workspaceId,provider,reference)=>decode(await call('ai/providers-credential-set',{workspaceId,provider,reference}),validProviderCredentialSetResponse),
    providersCredentialRemove:async(workspaceId,provider,reference)=>decode(await call('ai/providers-credential-remove',{workspaceId,provider,reference}),validProviderCredentialRemoveResponse),
    providersCredentialStatus:async(workspaceId,provider,reference)=>decode(await call('ai/providers-credential-status',{workspaceId,provider,reference}),validProviderCredentialStatusResponse),
    providersSessionAccess:async(workspaceId)=>decode(await call('ai/providers-session-access',{workspaceId}),validProviderSessionAccessResponse),
    startProjectIdeas:async(workspaceId,input)=>decode(await call('ai/ideas-generate-start',{workspaceId,...input}),validIdeasGenerateStartResponse),
    pollProjectIdeas:async(workspaceId,runId)=>decode(await call('ai/ideas-generate-poll',{workspaceId,runId}),validIdeasPollResponse),
    cancelProjectIdeas:async(workspaceId,runId)=>decode(await call('ai/ideas-generate-cancel',{workspaceId,runId}),validIdeasPollResponse),
    saveProjectIdeas:async(workspaceId,runId)=>decode(await call('ai/ideas-save',{workspaceId,runId}),validIdeasSaveResponse),
    startProjectOpportunities:async(workspaceId,input)=>decode(await call('ai/opportunities-generate-start',{workspaceId,...input}),validOpportunitiesGenerateStartResponse),
    pollProjectOpportunities:async(workspaceId,runId)=>decode(await call('ai/opportunities-generate-poll',{workspaceId,runId}),validOpportunitiesPollResponse),
    cancelProjectOpportunities:async(workspaceId,runId)=>decode(await call('ai/opportunities-generate-cancel',{workspaceId,runId}),validOpportunitiesPollResponse),
    saveProjectOpportunities:async(workspaceId,runId)=>decode(await call('ai/opportunities-save',{workspaceId,runId}),validOpportunitiesSaveResponse),
    startForgeSpec:async(workspaceId,input)=>decode(await call('ai/forge-spec-start',{workspaceId,...input}),validForgeStartResponse),
    pollForgeSpec:async(workspaceId,runId)=>decode(await call('ai/forge-spec-poll',{workspaceId,runId}),validForgePollResponse),
    cancelForgeSpec:async(workspaceId,runId)=>decode(await call('ai/forge-spec-cancel',{workspaceId,runId}),validForgePollResponse),
    saveForgeSpec:async(workspaceId,runId)=>decode(await call('ai/forge-spec-save',{workspaceId,runId}),validForgeSaveResponse),
    approveForgeSpec:async(workspaceId,input)=>decode(await call('ai/forge-spec-approve',{workspaceId,...input}),validForgeSpecApproveResponse),
    specApprovalStatus:async(workspaceId,input)=>decode(await call('ai/forge-spec-approval-status',{workspaceId,...input}),validForgeSpecApprovalStatusResponse),
    startForgeBuild:async(workspaceId,input)=>decode(await call('ai/forge-build-start',{workspaceId,...input}),validForgeStartResponse),
    pollForgeBuild:async(workspaceId,runId)=>decode(await call('ai/forge-build-poll',{workspaceId,runId}),validForgePollResponse),
    cancelForgeBuild:async(workspaceId,runId)=>decode(await call('ai/forge-build-cancel',{workspaceId,runId}),validForgePollResponse),
    saveForgeBuild:async(workspaceId,runId)=>decode(await call('ai/forge-build-save',{workspaceId,runId}),validForgeSaveResponse),
    exportForgePackage:async(workspaceId,artifactId,version)=>decode(await call('artifacts/forge-export',{workspaceId,artifactId,version}),validForgeExportResponse),
    saveProjectEvaluation:async(workspaceId,input)=>decode(await call('artifacts/evaluations-save',{workspaceId,...input}),validEvaluationsSaveResponse),
    getStandardRubric:async(workspaceId,projectId)=>decode(await call('artifacts/evaluations-standard-rubric',{workspaceId,projectId}),validStandardRubricResponse),
    vaultCommand:async(command,request)=>await call('vault/'+command,request) as Record<string,unknown>,
  };
}
export function httpClient(endpoint:string,token:string):Client {
  let url:URL;
  try{url=new URL(endpoint);}catch{throw new Error('Enter a local API address such as http://127.0.0.1:1451.');}
  if(url.protocol!=='http:'||url.hostname!=='127.0.0.1'||!url.port||url.username||url.password||url.pathname!=='/'||url.search||url.hash)throw new Error('Use an HTTP address on 127.0.0.1 with a port and no path.');
  if(!/^[a-f0-9]{64}$/.test(token))throw new Error('Enter the 64-character token from the current private pairing file.');
  return client(async(operation,request)=>{
    let response:Response;
    const path = operation.startsWith('jobs/') || operation.startsWith('memories/') || operation.startsWith('relationships/') || operation.startsWith('ai/') || operation.startsWith('vault/') || operation.startsWith('artifacts/')
      ? operation
      : operation === 'readiness'
      ? 'readiness'
      : 'workspaces/' + operation;
    try{response=await fetch(`${url.origin}/v1/${path}`,{method:'POST',credentials:'omit',cache:'no-store',redirect:'error',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},body:JSON.stringify({contractVersion:1,...request}),signal:AbortSignal.timeout(operation.startsWith('jobs/')||operation.startsWith('ai/')||operation.startsWith('vault/')?120000:15000)});}
    catch{throw new Error('Local core unavailable. Start the API, then retry or pair with its new session.');}
    let value:unknown;try{value=await response.json();}catch{throw new Error('The core returned an unreadable response. Try again.');}
    if(!response.ok)throw failure(value,operation);return value;
  });
}
export function nativeClient():Client {
  return client(async(operation,request)=>{
    try{
      if(operation.startsWith('jobs/'))return await invoke('job_command',{command:operation.slice(5),request:{contractVersion:1,...request}});
      if(operation.startsWith('memories/'))return await invoke('memory_command',{command:operation.slice(9),request:{contractVersion:1,...request}});
      if(operation.startsWith('relationships/'))return await invoke('relationship_command',{command:operation.slice(14),request:{contractVersion:1,...request}});
      if(operation.startsWith('ai/'))return await invoke('ai_command',{command:operation.slice(3),request:{contractVersion:1,...request}});
      if(operation.startsWith('artifacts/'))return await invoke('artifact_command',{command:operation.slice(10),request:{contractVersion:1,...request}});
      if(operation.startsWith('vault/'))return await invoke('vault_command',{command:operation.slice(6),request:{contractVersion:1,...request}});
      if(operation==='readiness')return await invoke('get_readiness',{request:{contractVersion:1}});
      return await invoke('workspace_command',{command:operation,request:{contractVersion:1,...request}});
    }
    catch(error){throw failure(error,operation);}
  });
}

export async function saveNativeExport(workspaceId:string,jobId:string,artifactId:string):Promise<boolean>{
  try {const saved=await invoke('save_export',{request:{contractVersion:1,workspaceId,jobId,artifactId,offset:0}});if(typeof saved!=='boolean')throw Error();return saved;}catch(error){throw failure(error);}
}

export async function nativeVaultCommand(command:VaultCommandName,request:Record<string,unknown>):Promise<unknown>{
  try{return await invoke('vault_command',{command,request:{contractVersion:1,...request}});}catch(error){throw failure(error,'vault/'+command);}
}
