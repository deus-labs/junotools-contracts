
# CW3-Timelock
A smart contract that relays execute function calls on other smart contracts with a predetermined minimum time delay.

## Instantiate
```rust
pub struct InstantiateMsg {
  pub admins: Option<Vec<String>>,
  pub proposers: Vec<String>,
  pub min_delay: Duration,
}
```
## Execute
```rust
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
```

## Query
```rust
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
```
