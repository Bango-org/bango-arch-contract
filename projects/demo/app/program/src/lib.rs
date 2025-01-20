use std::{cell::RefMut, collections::HashMap};

use arch_program::{
    account::AccountInfo,
    bitcoin::{absolute::LockTime, amount, consensus, transaction::Version, Transaction},
    entrypoint::ProgramResult,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    msg,
    program::{
        get_bitcoin_block_height, next_account_info, set_transaction_to_sign,
        validate_utxo_ownership,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    transaction_to_sign::TransactionToSign,
    utxo::UtxoMeta,
};
use borsh::{BorshDeserialize, BorshSerialize};
use arch_program::entrypoint;


use mint::{burn_tokens, initialize_mint, mint_tokens, InitializeMintInput};
use token_account::initialize_balance_account;
use transfer::{transfer_tokens, TransferInput};
use types::{*};

pub mod types;
pub mod errors;
pub mod mint;
pub mod token_account;
pub mod transfer;


entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello 1");

    let function_number = instruction_data[0];

    msg!("Function Called {}", function_number);

    let account_iter = &mut accounts.clone().iter();

    match function_number {
        1 => {
            msg!("Instruction: CreateEvent");

            let params = PredictionEventParams::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;

            let res = process_create_event(
                accounts,
                params.unique_id,
                params.expiry_timestamp,
                params.num_outcomes,
            );

            res
        }

        2 => {
            msg!("Instruction: CloseEvent");

            let params = ClosePredictionEventParams::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;

            let res = process_close_event(accounts, params.unique_id);

            res
        }

        3 => {
            msg!("Instruction: Bet on Event Buy");

            let params = BetOnPredictionEventParams::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;

            let res = process_buy_bet(
                accounts,
                params.unique_id,
                params.outcome_id,
                params.amount,
            );

            res
        }

        4 => {
            msg!("Instruction: Bet on Event Sell");

            let params = BetOnPredictionEventParams::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;

            let res = process_sell_bet(
                accounts,
                params.unique_id,
                params.outcome_id,
                params.amount,
            );

            res
        }


        5 => {
            /* -------------------------------------------------------------------------- */
            /*                               INITIALIZE MINT                              */
            /* -------------------------------------------------------------------------- */
            // 1 Account : (owned by program, uninitialized)
            msg!("Initializing Mint Account ");

            if accounts.len() != 1 {
                return Err(ProgramError::Custom(502));
            }

            let account = next_account_info(account_iter)?;

            let initialize_mint_input: InitializeMintInput =
                borsh::from_slice(&instruction_data[1..])
                    .map_err(|_e| ProgramError::InvalidArgument)?;

            initialize_mint(account, program_id, initialize_mint_input)?;
            Ok(())            
        }

        6 => {
            msg!("Mint TOkens");

            /* -------------------------------------------------------------------------- */
            /*                                 MINT TOKENS                                */
            /* -------------------------------------------------------------------------- */
            // 1 - Mint account ( owned by program and writable )
            // 2 - Balance account ( owned by program and writable )
            // 3 - Owner account( signer )
            if accounts.len() != 2 {
                return Err(ProgramError::Custom(502));
            }

            let token_account = next_account_info(account_iter)?;

            let owner_account = next_account_info(account_iter)?;

            let mint_params: MintTokenParams = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;
            
            mint_tokens(
                token_account,
                owner_account.key,
                mint_params.amount
            )?;
            
            Ok(())
        }



        7 => {
            msg!("Burn TOkens");

            /* -------------------------------------------------------------------------- */
            /*                                 Burn TOKENS                                */
            /* -------------------------------------------------------------------------- */
            // 1 - Mint account ( owned by program and writable )
            // 2 - Balance account ( owned by program and writable )
            // 3 - Owner account( signer )
            if accounts.len() != 2 {
                return Err(ProgramError::Custom(502));
            }

            let token_account = next_account_info(account_iter)?;

            let owner_account = next_account_info(account_iter)?;

            let mint_params: MintTokenParams = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;
            
            burn_tokens(
                token_account,
                owner_account.key,
                mint_params.amount
            )?;
            
            Ok(())
        }

        _ => Err(ProgramError::BorshIoError(String::from(
            "Invalid function call",
        ))),
    }
}

pub fn process_create_event(
    accounts: &[AccountInfo],
    unique_id: [u8; 32],
    expiry_timestamp: u32,
    num_outcomes: u8,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let event_account = next_account_info(accounts_iter)?;
    let creator_account = next_account_info(accounts_iter)?;

    msg!("Hello1 {}, {}", creator_account.is_signer, creator_account.is_executable);
    if !creator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut outcomes = Vec::new();
    for i in 0..num_outcomes {
        outcomes.push(Outcome {
            id: i,
            total_amount: 0,
            bets: HashMap::new(),
        });
    }

    let event = PredictionEvent {
        unique_id: unique_id,
        creator: creator_account.key.clone(),
        expiry_timestamp: expiry_timestamp,
        outcomes: outcomes,
        total_pool_amount: 0,
        status: EventStatus::Active,
        winning_outcome: None,
    };

    let data = event_account.try_borrow_mut_data()?;

    // fetch all events data
    let mut predictions_data = helper_deserialize_predictions(data)?;

    predictions_data.predictions.push(event);
    predictions_data.total_predictions += 1;

    helper_store_predictions(event_account, predictions_data)
}

pub fn process_close_event(
    accounts: &[AccountInfo],
    unique_id: [u8; 32],
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let event_account = next_account_info(accounts_iter)?;
    let creator_account = next_account_info(accounts_iter)?;

    if !creator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let data = event_account.try_borrow_mut_data()?;
    let mut predictions_data = helper_deserialize_predictions(data)?;

    let index = predictions_data
        .predictions
        .iter()
        .position(|x| x.unique_id == unique_id)
        .unwrap();

    predictions_data.predictions[index].status = EventStatus::Closed;
    predictions_data.total_predictions -= 1;

    helper_store_predictions(event_account, predictions_data)
}

pub fn helper_deserialize_predictions(
    data: RefMut<'_, &mut [u8]>,
) -> Result<Predictions, ProgramError> {
    msg!("Total bytes: {}", data.len());
    let predictions_data = if data.len() > 0 {
        Predictions::try_from_slice(&data).map_err(|e| {
            msg!("Error: Failed to deserialize event data {}", e.to_string());
            ProgramError::BorshIoError(String::from("Error: Failed to deserialize event data"))
        })?
    } else {
        Predictions {
            total_predictions: 0,
            predictions: Vec::new(),
        }
    };

    Ok(predictions_data)
}

pub fn helper_store_predictions(
    event_account: &AccountInfo<'_>,
    predictions_data: Predictions,
) -> Result<(), ProgramError> {
    let serialized_data = borsh::to_vec(&predictions_data)
        .map_err(|_| ProgramError::BorshIoError(String::from("Serailization failed")))?;
    let required_len = serialized_data.len();
    msg!("Serlized data length {}", required_len);

    if event_account.data_len() < required_len {
        event_account.realloc(required_len, false)?;
    }

    msg!("account size {}", event_account.data_len());

    event_account.data.borrow_mut()[..required_len].copy_from_slice(&serialized_data);

    Ok(())
}

pub fn process_buy_bet(
    accounts: &[AccountInfo],
    unique_id: [u8; 32],
    outcome_id: u8,
    amount: u64,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let event_account = next_account_info(accounts_iter)?;
    let token_account = next_account_info(accounts_iter)?;
    let better_account = next_account_info(accounts_iter)?;

    if !better_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut events = Predictions::try_from_slice(&event_account.data.borrow())
        .map_err(|_| ProgramError::BorshIoError(String::from("No event exists")))?;

    let event = events
        .predictions
        .iter_mut()
        .find(|p| p.unique_id == unique_id)
        .unwrap();

    if event.status != EventStatus::Active {
        return Err(ProgramError::BorshIoError(String::from("Event is closed.")));
    }

    let bet = Bet {
        user: better_account.key.clone(),
        event_id: event.unique_id,
        outcome_id,
        amount,
        timestamp: get_bitcoin_block_height() as i64,
        bet_type: BetType::BUY
    };

    let outcome = event
        .outcomes
        .iter_mut()
        .find(|outcome| outcome.id == outcome_id)
        .unwrap();

    let bets: Option<&mut Vec<Bet>> = outcome.bets.get_mut(&better_account.key);

    if let Some(bets) = bets {
        // You now have `bets`, which is a mutable reference to `Vec<Bet>`
        bets.push(bet);
    } else {
        outcome.bets.insert(better_account.key.clone(), vec![bet]).unwrap();
    }

    event
        .serialize(&mut *event_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    burn_tokens(token_account, better_account.key, amount).unwrap();

    Ok(())
}



pub fn process_sell_bet(
    accounts: &[AccountInfo],
    unique_id: [u8; 32],
    outcome_id: u8,
    amount: u64,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let event_account = next_account_info(accounts_iter)?;
    let token_account = next_account_info(accounts_iter)?;
    let better_account = next_account_info(accounts_iter)?;

    if !better_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut events = Predictions::try_from_slice(&event_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let event = events
        .predictions
        .iter_mut()
        .find(|p| p.unique_id == unique_id)
        .unwrap();

    if event.status != EventStatus::Active {
        return Err(ProgramError::BorshIoError(String::from("Event is closed.")));
    }

    let bet = Bet {
        user: better_account.key.clone(),
        event_id: event.unique_id,
        outcome_id,
        amount,
        timestamp: get_bitcoin_block_height() as i64,
        bet_type: BetType::SELL
    };

    let outcome = event
        .outcomes
        .iter_mut()
        .find(|outcome| outcome.id == outcome_id)
        .unwrap();

    let bets: Option<&mut Vec<Bet>> = outcome.bets.get_mut(&better_account.key);

    if let Some(bets) = bets {
        // You now have `bets`, which is a mutable reference to `Vec<Bet>`
        bets.push(bet);
    } else {
        outcome.bets.insert(better_account.key.clone(), vec![bet]).unwrap();
    }

    event
        .serialize(&mut *event_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    mint_tokens(token_account, better_account.key, amount).unwrap();

    Ok(())
}


