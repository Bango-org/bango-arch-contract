use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;

use arch_program::{
    pubkey::Pubkey,
    utxo::UtxoMeta,
};


#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Outcome {
    pub id: u8,
    pub total_amount: u64,
    pub bets: HashMap<Pubkey, Vec<Bet>>,
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

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Bet {
    pub user: Pubkey,
    pub event_id: [u8; 32],
    pub outcome_id: u8,
    pub amount: u64,
    pub timestamp: i64,
    pub bet_type: BetType
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Predictions {
    pub total_predictions: u32,
    pub predictions: Vec<PredictionEvent>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PredictionEventParams {
    pub unique_id: [u8; 32],
    pub expiry_timestamp: u32,
    pub num_outcomes: u8,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ClosePredictionEventParams {
    pub unique_id: [u8; 32],
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BetOnPredictionEventParams {
    pub unused_uid: [u8; 32],
    pub unique_id: [u8; 32],
    pub outcome_id: u8,
    pub amount: u64
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MintTokenParams {
    pub uid: [u8; 32],
    pub amount: u64
}


#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum EventStatus {
    Active,
    Closed,
    Resolved,
    Cancelled,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum BetType {
    SELL,
    BUY
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
