use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Empty balance")]
    EmptyBalance {},

    #[error("Insufficient amount sent")]
    InsufficientAmount {},

    #[error("Escrow already released")]
    EscrowAlreadyReleased {},

    #[error("Cannot release funds")]
    CannotReleaseFunds {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
