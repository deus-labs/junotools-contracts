use cosmwasm_std::{StdError};
use thiserror::Error;


#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Delay time not ended")]
    Unexpired {},

    #[error("Address {address:?} not found in Proposers")]
    NotFound {address: String},

    #[error("Can not delete executed proposals")]
    NotDeletable {},

    #[error("Proposers list already contains this proposer address")]
    AlreadyContainsProposerAddress {},

    #[error("Minimum Delay condition not satisfied.")]
    MinDelayNotSatisfied {},

    #[error("This operation already executed.")]
    Executed {},

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
