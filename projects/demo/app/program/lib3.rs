use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    system_instruction,
    program_pack::Pack,
    program_error::ProgramError,
    instruction::{AccountMeta, Instruction},
};
use borsh::{BorshSerialize, BorshDeserialize};
use std::convert::TryInto;

// Program-specific errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PredictionMarketError {
    InvalidInstruction,
    InsufficientFunds,
    EventAlreadyExists,
    EventNotFound,
    InvalidOutcome,
    EventNotResolved,
    EventAlreadyResolved,
}

impl From<PredictionMarketError> for ProgramError {
    fn from(e: PredictionMarketError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// Event status enum
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub enum EventStatus {
    Created,
    Active,
    Resolved,
    Cancelled,
}

// Prediction Event Structure
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PredictionEvent {
    pub unique_id: [u8; 32],
    pub creator: Pubkey,
    pub expiry_timestamp: u64,
    pub outcomes: Vec<String>,
    pub total_pool_amount: u64,
    pub status: EventStatus,
    pub winning_outcome: Option<String>,
    pub outcome_balances: Vec<u64>, // Track balance for each outcome
}

// Bet structure
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct Bet {
    pub event_id: [u8; 32],
    pub bettor: Pubkey,
    pub amount: u64,
    pub chosen_outcome: String,
}

// Instructions for the Prediction Market
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub enum PredictionMarketInstruction {
    /// Create a new prediction event
    /// Accounts expected:
    /// 0. `[signer, writable]` Event creator account
    /// 1. `[writable]` Event account to be created
    /// 2. `[]` System program
    CreateEvent {
        unique_id: [u8; 32],
        expiry_timestamp: u64,
        outcomes: Vec<String>,
    },

    /// Place a bet on a specific outcome
    /// Accounts expected:
    /// 0. `[signer, writable]` Bettor's account
    /// 1. `[writable]` Event account
    /// 2. `[]` System program
    PlaceBet {
        amount: u64,
        chosen_outcome: String,
    },

    /// Resolve an event with a winning outcome
    /// Accounts expected:
    /// 0. `[signer]` Event creator/resolver
    /// 1. `[writable]` Event account
    ResolveEvent {
        winning_outcome: String,
    },

    /// Claim winnings for a resolved event
    /// Accounts expected:
    /// 0. `[signer, writable]` Bettor's account
    /// 1. `[writable]` Event account
    ClaimWinnings {
        event_id: [u8; 32],
    },
}

// Program entrypoint
entrypoint!(process_instruction);

pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = PredictionMarketInstruction::try_from_slice(instruction_data)
        .map_err(|_| PredictionMarketError::InvalidInstruction)?;

    match instruction {
        PredictionMarketInstruction::CreateEvent { 
            unique_id, 
            expiry_timestamp, 
            outcomes 
        } => {
            msg!("Instruction: CreateEvent");
            create_event(program_id, accounts, unique_id, expiry_timestamp, outcomes)
        },
        PredictionMarketInstruction::PlaceBet { 
            amount, 
            chosen_outcome 
        } => {
            msg!("Instruction: PlaceBet");
            place_bet(program_id, accounts, amount, chosen_outcome)
        },
        PredictionMarketInstruction::ResolveEvent { 
            winning_outcome 
        } => {
            msg!("Instruction: ResolveEvent");
            resolve_event(program_id, accounts, winning_outcome)
        },
        PredictionMarketInstruction::ClaimWinnings { 
            event_id 
        } => {
            msg!("Instruction: ClaimWinnings");
            claim_winnings(program_id, accounts, event_id)
        },
    }
}

// Create a new prediction event
fn create_event(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    unique_id: [u8; 32],
    expiry_timestamp: u64,
    outcomes: Vec<String>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let creator = next_account_info(accounts_iter)?;
    let event_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Verify creator is signer
    if !creator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check if event already exists
    if event_account.data.borrow().len() > 0 {
        return Err(PredictionMarketError::EventAlreadyExists.into());
    }

    // Create prediction event
    let event = PredictionEvent {
        unique_id,
        creator: *creator.key,
        expiry_timestamp,
        outcomes: outcomes.clone(),
        total_pool_amount: 0,
        status: EventStatus::Created,
        winning_outcome: None,
        outcome_balances: vec![0; outcomes.len()],
    };

    // Serialize and store event data
    let serialized_event = event.try_to_vec()?;
    
    // Allocate space for the event
    let space = serialized_event.len();
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);

    // Create account with enough space and lamports
    invoke(
        &system_instruction::create_account(
            creator.key,
            event_account.key,
            lamports,
            space as u64,
            program_id
        ),
        &[creator.clone(), event_account.clone(), system_program.clone()]
    )?;

    // Copy serialized data to event account
    event_account.data.borrow_mut()[..space].copy_from_slice(&serialized_event);

    Ok(())
}

// Place a bet on an event
fn place_bet(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    chosen_outcome: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let bettor = next_account_info(accounts_iter)?;
    let event_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Verify bettor is signer
    if !bettor.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Deserialize event data
    let mut event = PredictionEvent::try_from_slice(&event_account.data.borrow())?;

    // Validate bet
    if event.status != EventStatus::Active {
        return Err(PredictionMarketError::EventNotFound.into());
    }

    // Check if outcome is valid
    let outcome_index = event.outcomes.iter()
        .position(|o| o == &chosen_outcome)
        .ok_or(PredictionMarketError::InvalidOutcome)?;

    // Transfer bet amount from bettor to event account
    invoke(
        &system_instruction::transfer(
            bettor.key,
            event_account.key,
            amount
        ),
        &[bettor.clone(), event_account.clone(), system_program.clone()]
    )?;

    // Update event data
    event.total_pool_amount += amount;
    event.outcome_balances[outcome_index] += amount;

    // Serialize and store updated event
    let serialized_event = event.try_to_vec()?;
    event_account.data.borrow_mut()[..serialized_event.len()].copy_from_slice(&serialized_event);

    Ok(())
}

// Resolve an event with a winning outcome
fn resolve_event(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    winning_outcome: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let resolver = next_account_info(accounts_iter)?;
    let event_account = next_account_info(accounts_iter)?;

    // Deserialize event data
    let mut event = PredictionEvent::try_from_slice(&event_account.data.borrow())?;

    // Validate resolver and event status
    if *resolver.key != event.creator {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if event.status != EventStatus::Active {
        return Err(PredictionMarketError::EventAlreadyResolved.into());
    }

    // Validate winning outcome
    if !event.outcomes.contains(&winning_outcome) {
        return Err(PredictionMarketError::InvalidOutcome.into());
    }

    // Update event with winning outcome
    event.status = EventStatus::Resolved;
    event.winning_outcome = Some(winning_outcome);

    // Serialize and store updated event
    let serialized_event = event.try_to_vec()?;
    event_account.data.borrow_mut()[..serialized_event.len()].copy_from_slice(&serialized_event);

    Ok(())
}

// Claim winnings for a resolved event
fn claim_winnings(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    event_id: [u8; 32],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let winner = next_account_info(accounts_iter)?;
    let event_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Deserialize event data
    let event = PredictionEvent::try_from_slice(&event_account.data.borrow())?;

    // Validate event status and winning outcome
    if event.status != EventStatus::Resolved {
        return Err(PredictionMarketError::EventNotResolved.into());
    }

    let winning_outcome = event.winning_outcome
        .as_ref()
        .ok_or(PredictionMarketError::EventNotResolved)?;

    // Calculate winner's share
    let winner_outcome_balance = event.outcome_balances[
        event.outcomes.iter().position(|o| o == winning_outcome)
            .ok_or(PredictionMarketError::InvalidOutcome)?
    ];

    let total_winning_pool = event.outcome_balances[
        event.outcomes.iter().position(|o| o == winning_outcome)
            .ok_or(PredictionMarketError::InvalidOutcome)?
    ];

    // Proportional payout calculation
    let payout = (winner_outcome_balance * event.total_pool_amount) / total_winning_pool;

    // Transfer winnings
    invoke(
        &system_instruction::transfer(
            event_account.key,
            winner.key,
            payout
        ),
        &[event_account.clone(), winner.clone(), system_program.clone()]
    )?;

    Ok(())
}

// Required to support creating instructions from outside the program
pub fn create_create_event_instruction(
    program_id: Pubkey,
    creator: Pubkey,
    unique_id: [u8; 32],
    expiry_timestamp: u64,
    outcomes: Vec<String>,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(creator, true),
            AccountMeta::new_readonly(SystemProgram::id(), false),
        ],
        data: PredictionMarketInstruction::CreateEvent {
            unique_id,
            expiry_timestamp,
            outcomes,
        }.try_to_vec().unwrap(),
    }
}

// In a real-world scenario, you'd add more comprehensive error handling,
// more sophisticated payout mechanisms, and additional security checks