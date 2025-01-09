use std::{cell::RefMut, collections::HashMap};

use arch_program::{
    account::AccountInfo,
    bitcoin::{absolute::LockTime, consensus, transaction::Version, Transaction},
    entrypoint::{ProgramResult},
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


use mint::{initialize_mint, mint_tokens, InitializeMintInput, MintInput};
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

    msg!("Hello 1");

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
            msg!("Instruction: Bet on Event");

            let params = BetOnPredictionEventParams::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;

            let res = process_place_bet(
                accounts,
                params.unique_id,
                params.outcome_id,
                params.amount,
                params.tx_hex,
                params.utxo,
            );

            res
        }


        4 => {
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

        5 => {
            /* -------------------------------------------------------------------------- */
            /*                         INITIALIZE BALANCE ACCOUNT                         */
            /* -------------------------------------------------------------------------- */
            // No instruction data needed, only 3 accounts
            // 1 - Token balance owner (signer, writable)
            // 2 - Mint account (owned by program and writable)
            // 3 - Supplied account (owned by program, uninitialized )
            if accounts.len() != 3 {
                return Err(ProgramError::Custom(502));
            }

            let owner_account = next_account_info(account_iter)?;

            let mint_account = next_account_info(account_iter)?;

            let balance_account = next_account_info(account_iter)?;

            initialize_balance_account(owner_account, mint_account, balance_account, program_id)?;
            Ok(())
        }

        6 => {
            /* -------------------------------------------------------------------------- */
            /*                                 MINT TOKENS                                */
            /* -------------------------------------------------------------------------- */
            // 1 - Mint account ( owned by program and writable )
            // 2 - Balance account ( owned by program and writable )
            // 3 - Owner account( signer )
            if accounts.len() != 3 {
                return Err(ProgramError::Custom(502));
            }

            let mint_account = next_account_info(account_iter)?;

            let balance_account = next_account_info(account_iter)?;

            let owner_account = next_account_info(account_iter)?;

            let mint_input: MintInput = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;

            mint_tokens(
                balance_account,
                mint_account,
                owner_account,
                program_id,
                mint_input,
            )?;
            Ok(())
        }

        7 => {
            /* -------------------------------------------------------------------------- */
            /*                               TRANSFER TOKENS                              */
            /* -------------------------------------------------------------------------- */
            // 1 - Owner Account ( is_signer )
            // 2 - Mint Account ( writable and owned by program )
            // 3 - Sender Account ( writable and owned by program, balance owner is Account 1 )
            // 4 - Receiver Account ( writable and owned by program )

            if accounts.len() != 4 {
                return Err(ProgramError::Custom(502));
            }

            let owner_account = next_account_info(account_iter)?;

            let mint_account = next_account_info(account_iter)?;

            let sender_account = next_account_info(account_iter)?;

            let receiver_account = next_account_info(account_iter)?;

            let transfer_input: TransferInput = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;

            transfer_tokens(
                owner_account,
                mint_account,
                sender_account,
                receiver_account,
                program_id,
                transfer_input,
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
    msg!("Hello 1");

    let data = event_account.try_borrow_mut_data()?;
    msg!("Hello 1");

    // fetch all events data
    let mut predictions_data = helper_deserialize_predictions(data)?;
    msg!("Hello 1");

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

pub fn process_place_bet(
    accounts: &[AccountInfo],
    unique_id: [u8; 32],
    outcome_id: u8,
    amount: u64,
    tx_hex: Vec<u8>,
    utxo: UtxoMeta,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let event_account = next_account_info(accounts_iter)?;
    let better_account = next_account_info(accounts_iter)?;

    if !better_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut events = Predictions::try_from_slice(&event_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let mut event = events
        .predictions
        .iter_mut()
        .find(|p| p.unique_id == unique_id)
        .unwrap();

    if event.status != EventStatus::Active {
        return Err(ProgramError::BorshIoError(String::from("Event is closed.")));
    }

    if !validate_utxo_ownership(better_account.utxo, better_account.key) {
        return Err(ProgramError::InvalidArgument);
    }

    if better_account.utxo.clone() != UtxoMeta::from_slice(&[0; 36]) {
        msg!("UTXO {:?}", better_account.utxo.clone());
        return Err(ProgramError::BorshIoError(String::from("No UTXO Passed")));
    }

    let fees_tx: Transaction = consensus::deserialize(&tx_hex).unwrap();

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![],
        output: vec![],
    };

    add_state_transition(&mut tx, event_account);
    tx.input.push(fees_tx.input[0].clone());

    let tx_to_sign = TransactionToSign {
        tx_bytes: &consensus::serialize(&tx),
        inputs_to_sign: &[InputToSign {
            index: 0,
            signer: event_account.key.clone(),
        }],
    };

    msg!("tx_to_sign{:?}", tx_to_sign);

    set_transaction_to_sign(accounts, tx_to_sign);

    let bet = Bet {
        user: better_account.key.clone(),
        event_id: event.unique_id,
        outcome_id,
        amount,
        tx_hex,
        utxo,
        timestamp: get_bitcoin_block_height() as i64,
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
        outcome.bets.insert(better_account.key.clone(), vec![bet]);
    }

    event
        .serialize(&mut *event_account.data.borrow_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(())
}
