use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
   
    #[error("Not enough token")]
    InSufficientToken {},

    #[error("Wrong Portion Error")]
    WrongPortionError {},

    #[error("Sum of portion is not 1 ")]
    PortionError{},

    #[error("NFT contract Error")]
    WrongNFTContractError{},
    
    #[error("Token contract Error")]
    WrongTokenContractError{},

     #[error("No data")]
    NoData{},

    #[error("Not Enough Funds")]
    NotEnoughFunds {},

    #[error("Too Much Funds")]
    TooMuchFunds {},

    #[error("Wrong Collection")]
    WrongCollection {},


    #[error("Escrow not expired")]
    NotExpired {},
}
