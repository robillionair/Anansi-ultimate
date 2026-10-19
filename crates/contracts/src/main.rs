#![recursion_limit = "256"]

use anansi_core::jobs::{
    ArtifactChunk, ControlJobRequest, EnqueueExportRequest, Job, JobDetails, JobList,
    ListJobsRequest, ReadArtifactRequest,
};
use anansi_core::memory::{
    MemoryDetails, MemoryDraft, MemoryList, MemoryListRequest, MemoryMetadata, MemoryReadRequest,
    MemoryWriteRequest,
};
use anansi_core::relationships::{
    CreateRelationshipRequest, DeleteRelationshipRequest, ListRelationshipsRequest,
    RelationshipDetails, RelationshipDraft, RelationshipList, RelationshipRecord,
};
use anansi_core::workspace::{
    CreateWorkspaceRequest, OpenWorkspaceRequest, UpdateSettingsRequest, WorkspaceDetails,
    WorkspaceList,
};
use anansi_core::{CommandError, Readiness, ReadinessRequest};
fn main() {
    let schemas = serde_json::json!({
        "UploadBegin":schemars::schema_for!(anansi_core::backup_upload::UploadBegin),
        "UploadPart":schemars::schema_for!(anansi_core::backup_upload::UploadPart),
        "UploadStatus":schemars::schema_for!(anansi_core::backup_upload::UploadStatus),
        "SnapshotPreview":schemars::schema_for!(anansi_core::snapshots::SnapshotPreview),
        "RestoreResult":schemars::schema_for!(anansi_core::restore::RestoreResult),
        "BackupSchedule":schemars::schema_for!(anansi_core::backup_commands::BackupSchedule),
        "BackupRequest":schemars::schema_for!(anansi_core::backup_commands::BackupRequest),
        "SnapshotRequest":schemars::schema_for!(anansi_core::backup_commands::SnapshotRequest),
        "RestoreRequest":schemars::schema_for!(anansi_core::backup_commands::RestoreRequest),
        "ScheduleRequest":schemars::schema_for!(anansi_core::backup_commands::ScheduleRequest),
        "ScheduleReadRequest":schemars::schema_for!(anansi_core::backup_commands::ScheduleReadRequest),
        "Job":schemars::schema_for!(Job),
        "JobDetails":schemars::schema_for!(JobDetails),
        "JobList":schemars::schema_for!(JobList),
        "ArtifactChunk":schemars::schema_for!(ArtifactChunk),
        "EnqueueExportRequest":schemars::schema_for!(EnqueueExportRequest),
        "ListJobsRequest":schemars::schema_for!(ListJobsRequest),
        "ControlJobRequest":schemars::schema_for!(ControlJobRequest),
        "ReadArtifactRequest":schemars::schema_for!(ReadArtifactRequest),
        "MemoryDraft":schemars::schema_for!(MemoryDraft),
        "MemoryMetadata":schemars::schema_for!(MemoryMetadata),
        "MemoryDetails":schemars::schema_for!(MemoryDetails),
        "MemoryList":schemars::schema_for!(MemoryList),
        "MemoryListRequest":schemars::schema_for!(MemoryListRequest),
        "MemoryReadRequest":schemars::schema_for!(MemoryReadRequest),
        "MemoryWriteRequest":schemars::schema_for!(MemoryWriteRequest),
        "RelationshipDraft":schemars::schema_for!(RelationshipDraft),
        "RelationshipRecord":schemars::schema_for!(RelationshipRecord),
        "RelationshipDetails":schemars::schema_for!(RelationshipDetails),
        "RelationshipList":schemars::schema_for!(RelationshipList),
        "CreateRelationshipRequest":schemars::schema_for!(CreateRelationshipRequest),
        "DeleteRelationshipRequest":schemars::schema_for!(DeleteRelationshipRequest),
        "ListRelationshipsRequest":schemars::schema_for!(ListRelationshipsRequest),
        "Readiness":schemars::schema_for!(Readiness),
        "ReadinessRequest":schemars::schema_for!(ReadinessRequest),
        "CommandError":schemars::schema_for!(CommandError),
        "WorkspaceDetails":schemars::schema_for!(WorkspaceDetails),
        "WorkspaceList":schemars::schema_for!(WorkspaceList),
        "CreateWorkspaceRequest":schemars::schema_for!(CreateWorkspaceRequest),
        "OpenWorkspaceRequest":schemars::schema_for!(OpenWorkspaceRequest),
        "UpdateSettingsRequest":schemars::schema_for!(UpdateSettingsRequest),
        "AiDiscoveryRequest":schemars::schema_for!(anansi_core::ai_discovery::AiDiscoveryRequest),
        "AiDiscoveryResponse":schemars::schema_for!(anansi_core::ai_discovery::AiDiscoveryResponse),
        "HardwareProfile":schemars::schema_for!(anansi_core::ai_discovery::HardwareProfile),
        "DiscoveredRuntime":schemars::schema_for!(anansi_core::ai_discovery::DiscoveredRuntime),
        "ModelRecommendation":schemars::schema_for!(anansi_core::ai_discovery::ModelRecommendation),
        "AskRequest":schemars::schema_for!(anansi_core::ask::AskRequest),
        "AskResponse":schemars::schema_for!(anansi_core::ask::AskResponse),
        "GroundedCitation":schemars::schema_for!(anansi_core::ask::GroundedCitation),
        "DisclosureManifest":schemars::schema_for!(anansi_core::ask::DisclosureManifest)
    });
    println!("{}", serde_json::to_string_pretty(&schemas).unwrap());
}
