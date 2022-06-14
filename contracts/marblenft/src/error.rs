use cosmwasm_std::StdError;
use thiserror::Error;
use cw_utils::{Expiration, Scheduled};

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("NotSupported")]
    NotSupported {},

    #[error("NotMinted")]
    NotMinted {},

    #[error("InvalidCw20Token")]
    InvalidCw20Token {},

    #[error("InvalidUnitPrice")]
    InvalidUnitPrice {},

    #[error("InvalidMaxTokens")]
    InvalidMaxTokens {},

    #[error("SoldOut")]
    SoldOut {},

    #[error("OnlyNativeSell")]
    OnlyNativeSell {},

    #[error("UnauthorizedTokenContract")]
    UnauthorizedTokenContract {},

    #[error("Uninitialized")]
    Uninitialized {},

    #[error("CountNotMatch")]
    CountNotMatch {},

    #[error("WrongPaymentAmount")]
    WrongPaymentAmount {},

    #[error("InvalidTokenReplyId")]
    InvalidTokenReplyId {},

    #[error("Cw721AlreadyLinked")]
    Cw721AlreadyLinked {},

    #[error("Incorrect funds")]
    IncorrectFunds {},

    #[error("Verification failed")]
    VerificationFailed {},

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },

    #[error("Airdrop stage {stage} expired at {expiration}")]
    StageExpired { stage: u8, expiration: Expiration },

    #[error("Airdrop stage {stage} not expired yet")]
    StageNotExpired { stage: u8, expiration: Expiration },

    #[error("Airdrop stage {stage} begins at {start}")]
    StageNotBegun { stage: u8, start: Scheduled },

    #[error("Insufficient Tokens")]
    InsufficientFund {},

    #[error("AlreadySold")]
    AlreadySold {},

    #[error("Already claimed")]
    Claimed {},

    #[error("Wrong length")]
    WrongLength {},

    #[error("InsufficientRoyalty")]
    InsufficientRoyalty {},
}
