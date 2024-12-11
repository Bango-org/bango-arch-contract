use std::cell::RefMut;

use arch_program::{
    account::AccountInfo,
    bitcoin::hex::DisplayHex,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{get_bitcoin_block_height, next_account_info, validate_utxo_ownership},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum EventStatus {
    Active,
    Closed,
    Resolved,
    Cancelled,
}

pub enum PredictionMarketError {
    InvalidInstruction,
    InsufficientFunds,
    EventAlreadyExists,
    EventNotFound,
    InvalidOutcome,
    EventNotResolved,
    EventAlreadyResolved,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Outcome {
    pub id: u8,
    pub total_amount: u64,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct PredictionEvent {
    pub unique_id: [u8; 32],
    pub creator: Pubkey,
    pub expiry_timestamp: u32,
    pub outcomes: Vec<Outcome>,
    pub total_pool_amount: u64,
    pub status: EventStatus,
    pub winning_outcome: Option<u8>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Bet {
    pub user: Pubkey,
    pub event_id: [u8; 32],
    pub outcome_id: u8,
    pub amount: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PredictionEventParams {
    pub unique_id: [u8; 32],
    pub expiry_timestamp: u32,
    pub num_outcomes: u8,
}


#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Predictions {
    pub total_predictions: u32,
    pub predictions: Vec<PredictionEvent>,
}


entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello 1");

    let function_number = instruction_data[0];

    msg!("Hello 1");

    match function_number {
        1 => {
            msg!("Instruction: CreateEvent");

            let params = PredictionEventParams::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;

            let res = process_create_event(program_id, accounts, params.unique_id, params.expiry_timestamp, params.num_outcomes);

            res
        },

        _ => {
            Err(ProgramError::BorshIoError(String::from("Invalid function call")))
        }
    }
}


pub fn process_create_event(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    unique_id: [u8; 32],
    expiry_timestamp: u32,
    num_outcomes: u8,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let event_account = next_account_info(accounts_iter)?;
    let creator_account = next_account_info(accounts_iter)?;

    if !creator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut outcomes = Vec::new();
    for i in 0..num_outcomes {
        outcomes.push(Outcome {
            id: i,
            total_amount: 0,
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

pub fn process_close_event(
    program_id: &Pubkey,
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

    let index = predictions_data.predictions.iter().position(|x| x.unique_id == unique_id).unwrap();

    predictions_data.predictions[index].status = EventStatus::Closed;
    predictions_data.total_predictions -= 1;

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


pub fn helper_deserialize_predictions(data:  RefMut<'_, &mut [u8]>) -> Result<Predictions, ProgramError> {

    let predictions_data = if data.len() > 0 {
        Predictions::try_from_slice(&data).map_err(|e| {
            msg!("Error: Failed to deserialize event data {}", e.to_string());
            ProgramError::BorshIoError(String::from("Error: Failed to deserialize event data"))
        })?
    } else {
        Predictions {
            total_predictions: 1,
            predictions: Vec::new()
        }
    };


    Ok(predictions_data)
}





// pub fn process_place_bet(
//     program_id: &Pubkey,
//     accounts: &[AccountInfo],
//     event_id: [u8; 32],
//     outcome_id: u8,
//     amount: u64,
// ) -> Result<(), ProgramError> {
//     let accounts_iter = &mut accounts.iter();
//     let event_account = next_account_info(accounts_iter)?;
//     let better_account = next_account_info(accounts_iter)?;
//     let bet_account = next_account_info(accounts_iter)?;

//     if !better_account.is_signer {
//         return Err(ProgramError::MissingRequiredSignature);
//     }

//     let mut event = PredictionEvent::try_from_slice(&event_account.data.borrow())
//         .map_err(|_| ProgramError::InvalidAccountData)?;

//     if event.status != EventStatus::Active {
//         return Err(ProgramError::InvalidAccountData);
//     }

//     if !validate_utxo_ownership(better_account.utxo, better_account.key) {
//         return Err(ProgramError::InvalidArgument);
//     }

//     if let Some(outcome) = event.outcomes.get_mut(outcome_id as usize) {
//         outcome.total_amount += amount;
//         event.total_pool_amount += amount;
//     } else {
//         return Err(ProgramError::InvalidArgument);
//     }

//     let bet = Bet {
//         user: *better_account.key,
//         event_id,
//         outcome_id,
//         amount,
//         timestamp: get_bitcoin_block_height() as i64,
//     };

//     bet.serialize(&mut *bet_account.data.borrow_mut())
//         .map_err(|_| ProgramError::InvalidAccountData)?;

//     event.serialize(&mut *event_account.data.borrow_mut())
//         .map_err(|_| ProgramError::InvalidAccountData)?;

//     Ok(())
// }
