use crate::{
    command::Command,
    replay::StateVersion,
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaintenanceCommand {
    Inspect {},
    Enter {
        expected: StateVersion,
    },
    SelectFolder {
        root: String,
        expected: StateVersion,
    },
    Prepare {
        copy: String,
        expected: StateVersion,
    },
    Apply {
        plan: String,
        expected: StateVersion,
    },
    Continue {
        plan: String,
        expected: StateVersion,
    },
    Cancel {
        plan: String,
        expected: StateVersion,
    },
    Rollback {
        plan: String,
        expected: StateVersion,
    },
    Retry {
        expected: StateVersion,
    },
    DeleteProblem {
        problem: String,
        expected: StateVersion,
    },
    IssueAccess {
        label: String,
        verifier: [u8; 32],
        writable: bool,
        expires_ms: Option<u64>,
        expected: StateVersion,
    },
    RevokeAccess {
        subject: String,
        expected: StateVersion,
    },
}
impl MaintenanceCommand {
    pub fn expected(&self) -> Option<StateVersion> {
        match self {
            Self::Inspect {} => None,
            Self::Enter { expected }
            | Self::SelectFolder { expected, .. }
            | Self::Prepare { expected, .. }
            | Self::Apply { expected, .. }
            | Self::Continue { expected, .. }
            | Self::Cancel { expected, .. }
            | Self::Rollback { expected, .. }
            | Self::Retry { expected }
            | Self::DeleteProblem { expected, .. }
            | Self::IssueAccess { expected, .. }
            | Self::RevokeAccess { expected, .. } => Some(*expected),
        }
    }
}
impl Command for MaintenanceCommand {
    const NAME: &'static str = "maintenance.execute";
    type Output = MaintenanceView;
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPhase {
    Prepared,
    Applying,
    Restored,
    Reverting,
    Reverted,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoveryPlan {
    pub id: String,
    pub copy: String,
    pub spaces: usize,
    pub created_at_unix_ms: u64,
    pub phase: RecoveryPhase,
    pub problem: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FullCopy {
    pub id: String,
    pub created_at_unix_ms: u64,
    pub spaces: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceAccess {
    pub subject: String,
    pub label: String,
    pub writable: bool,
    pub expires_ms: Option<u64>,
    pub revoked: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceView {
    pub version: StateVersion,
    pub normal_ready: bool,
    pub reason: Option<String>,
    pub owner: bool,
    pub writable: bool,
    pub folder: Option<String>,
    pub folder_available: bool,
    pub copies: Vec<FullCopy>,
    pub plan: Option<RecoveryPlan>,
    pub problems: Vec<String>,
    pub access: Vec<MaintenanceAccess>,
    pub issued_subject: Option<String>,
    pub endpoint: Option<String>,
}
#[utoipa::path(post, path = "/v1/commands/maintenance.execute", operation_id = "maintenance_execute", request_body = CommandRequest<MaintenanceCommand>, responses((status = 200, body = CommandResponse<MaintenanceView>)), security(("access_key" = [])))]
pub fn execute_contract() {}
