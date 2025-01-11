use std::collections::HashMap;

use arch_program::{account::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TokenMintDetails {
    owner: [u8; 32],
    pub status: MintStatus,
    pub supply: u64,             // in lowest denomination
    pub circulating_supply: u64, // in lowest denomination
    pub ticker: String,
    pub decimals: u8,
    token_metadata: HashMap<String, [u8; 32]>,

    pub balances: HashMap<Pubkey, u64>,
}

impl TokenMintDetails {
    pub fn new(
        input: InitializeMintInput,
        status: MintStatus,
        token_metadata: HashMap<String, [u8; 32]>,
    ) -> Self {
        TokenMintDetails {
            owner: input.owner,
            status,
            supply: input.supply,
            circulating_supply: 0,
            ticker: input.ticker,
            decimals: input.decimals,
            token_metadata,
            balances: HashMap::new(),
        }
    }
}
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Eq, PartialEq)]
pub enum MintStatus {
    Ongoing,
    Finished,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct InitializeMintInput {
    owner: [u8; 32],
    supply: u64, // in lowest denomination
    ticker: String,
    decimals: u8,
}
impl InitializeMintInput {
    pub fn new(owner: [u8; 32], supply: u64, ticker: String, decimals: u8) -> Self {
        InitializeMintInput {
            owner,
            supply,
            ticker,
            decimals,
        }
    }
}

pub(crate) fn initialize_mint(
    account: &AccountInfo<'_>,
    program_id: &Pubkey,
    mint_input: InitializeMintInput,
) -> Result<(), ProgramError> {
    if !account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let mint_initial_details =
        TokenMintDetails::new(mint_input, MintStatus::Ongoing, HashMap::new());

    let serialized_mint_details = borsh::to_vec(&mint_initial_details)
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    if !serialized_mint_details.is_empty() {
        account.realloc(serialized_mint_details.len(), true)?;
    }

    account
        .data
        .try_borrow_mut()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .copy_from_slice(&serialized_mint_details);

    Ok(())
}

pub(crate) fn mint_tokens(
    token_account: &AccountInfo<'_>,
    mint_address: &Pubkey,
    amount: u64,
) -> Result<(), ProgramError> {
    let mut token = TokenMintDetails::try_from_slice(&token_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let token_balance = token.balances.get(mint_address);

    match token_balance {
        Some(balance) => {
            token
                .balances
                .insert(mint_address.clone(), *balance + amount);
        }
        None => {
            token.balances.insert(mint_address.clone(), amount);
        }
    }

    let serialized_mint_details =
        borsh::to_vec(&token).map_err(|e| ProgramError::BorshIoError(e.to_string()))?;


    if token_account.data_len() < serialized_mint_details.len() {
        token_account.realloc(serialized_mint_details.len(), true)?;
    }

    token_account
        .data
        .try_borrow_mut()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .copy_from_slice(&serialized_mint_details);

    Ok(())
}



pub(crate) fn burn_tokens(
    token_account: &AccountInfo<'_>,
    mint_address: &Pubkey,
    amount: u64,
) -> Result<(), ProgramError> {
    let mut token = TokenMintDetails::try_from_slice(&token_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let token_balance = token.balances.get(mint_address);

    match token_balance {
        Some(balance) => {

            if *balance < amount {
                return Err(ProgramError::BorshIoError(String::from(
                    "Insufficient Balance!",
                )));
            }

            token
                .balances
                .insert(mint_address.clone(), *balance - amount);
        }
        None => {
            return Err(ProgramError::BorshIoError(String::from(
                "Account Not Exists!",
            )));
        }
    }

    let serialized_mint_details =
        borsh::to_vec(&token).map_err(|e| ProgramError::BorshIoError(e.to_string()))?;


    if token_account.data_len() < serialized_mint_details.len() {
        token_account.realloc(serialized_mint_details.len(), true)?;
    }

    token_account
        .data
        .try_borrow_mut()
        .map_err(|_e| ProgramError::AccountBorrowFailed)?
        .copy_from_slice(&serialized_mint_details);

    Ok(())
}
