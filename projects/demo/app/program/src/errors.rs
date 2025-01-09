use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum FungibleTokenError {
    InsufficientBalance,
    MintOver,
    NotEnoughRemainingMintableTokens,
}