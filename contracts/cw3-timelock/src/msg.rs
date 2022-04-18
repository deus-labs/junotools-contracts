use crate::state::{Operation, OperationStatus};
use cosmwasm_std::{Addr, Binary, Uint64};
use cw_utils::{Duration, Scheduled};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admins: Option<Vec<String>>,
    pub proposers: Vec<String>,
    pub min_delay: Duration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Schedule {
        target_address: String,
        data: Binary,
        title: String,
        description: String,
        execution_time: Scheduled,
        executors: Option<Vec<String>>,
    },

    Cancel {
        operation_id: Uint64,
    },

    Execute {
        operation_id: Uint64,
    },

    RevokeAdmin {
        admin_address: String,
    },

    AddProposer {
        proposer_address: String,
    },

    RemoveProposer {
        proposer_address: String,
    },

    UpdateMinDelay {
        new_delay: Duration,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetOperationStatus {
        operation_id: Uint64,
    },

    GetExecutionTime {
        operation_id: Uint64,
    },

    GetAdmins {},

    GetOperations {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    GetMinDelay {},

    GetProposers {},

    GetExecutors {
        operation_id: Uint64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OperationResponse {
    pub id: Uint64,
    pub status: OperationStatus,
    pub proposer: Addr,
    pub executors: Option<Vec<Addr>>,
    pub execution_time: Scheduled,
    pub target: Addr,
    pub data: Binary,
    pub title: String,
    pub description: String,
}

impl Into<OperationResponse> for Operation {
    fn into(self) -> OperationResponse {
        OperationResponse {
            id: self.id,
            status: self.status,
            proposer: self.proposer,
            executors: self.executors,
            execution_time: self.execution_time,
            target: self.target,
            data: self.data,
            title: self.title,
            description: self.description,
        }
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[allow(non_snake_case)]
pub struct OperationListResponse {
    pub operationList: Vec<OperationResponse>,
}
