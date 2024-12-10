use arch_program::{
    account::AccountInfo, bitcoin::hex::DisplayHex, entrypoint, msg, program::{get_bitcoin_block_height, next_account_info, validate_utxo_ownership}, program_error::ProgramError, pubkey::Pubkey
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum EventStatus {
    Active,
    Resolved,
    Cancelled,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Outcome {
    pub id: u8,
    pub total_amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
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

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum PredictionMarketInstruction {
    CreateEvent {
        unique_id: [u8; 32],
        expiry_timestamp: i64,
        num_outcomes: u8,
    },
    PlaceBet {
        event_id: [u8; 32],
        outcome_id: u8,
        amount: u64,
    },
    ResolveEvent {
        event_id: [u8; 32],
        winning_outcome: u8,
    },
    ClaimWinnings {
        event_id: [u8; 32],
    },
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let event_account = next_account_info(accounts_iter)?;
    let creator_account = next_account_info(accounts_iter)?;

    msg!("Hello1");

    // let unique_id_bytes: &[u8] = &instruction_data[0..32];
    // let extracted_string = String::from_utf8_lossy(unique_id_bytes).to_string();


    // let timestamp_bytes: &[u8] = &instruction_data[32..36]; // u32 is 4 bytes, starts at index 32
    // let timestamp = u32::from_le_bytes(timestamp_bytes.try_into().expect("Invalid u32 slice"));

    // let num_of_outcomes = &instruction_data[36];

    // msg!("title {}, timestamp: {}, outcomes: {}", extracted_string,timestamp, num_of_outcomes);


    if !creator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("Hello2");

    let params = PredictionEventParams::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("timestamp: {}",  params.expiry_timestamp);
    msg!("Hello3");

    let mut outcomes = Vec::new();
    for i in 0..params.num_outcomes {
        outcomes.push(Outcome {
            id: i,
            total_amount: 0,
        });
    }
    msg!("Hello4");

    let event = PredictionEvent {
        unique_id: params.unique_id,
        creator: creator_account.key.clone(),
        expiry_timestamp: params.expiry_timestamp,
        outcomes: outcomes,
        total_pool_amount: 0,
        status: EventStatus::Active,
        winning_outcome: None,
    };
 
    msg!("Hello5");

    
    let serialized_data = borsh::to_vec(&event).map_err(|_| ProgramError::BorshIoError(String::from("Serailization failed")))?;
    let required_len = serialized_data.len();
    msg!("Serlized ddata length {}", required_len);

    if event_account.data_len() < required_len {
        event_account.realloc(required_len + 10, false)?;
    }

    msg!("account size {}", event_account.data_len());

    event_account.data.borrow_mut()[..required_len].copy_from_slice(&serialized_data);

    msg!("Hello6");


    Ok(())
}

// pub fn process_create_event(
//     program_id: &Pubkey,
//     accounts: &[AccountInfo],
//     unique_id: [u8; 32],
//     expiry_timestamp: i64,
//     num_outcomes: u8,
// ) -> Result<(), ProgramError> {

// }

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
