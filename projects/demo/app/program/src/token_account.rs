use arch_program::{account::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::mint::TokenMintDetails;

#[derive(Clone, Copy, BorshSerialize, BorshDeserialize, Debug)]
pub struct TokenBalance {
    pub owner: [u8; 32],
    pub mint_account: [u8; 32],
    pub current_balance: u64, // in smallest denomination of token
}

impl TokenBalance {
    pub fn new(owner: [u8; 32], mint_account: [u8; 32]) -> Self {
        TokenBalance {
            owner,
            mint_account,
            current_balance: 0,
        }
    }

    pub fn increase_balance(
        &mut self,
        amount: u64, // in smallest denomination of token
        token_mint_details: &TokenMintDetails,
    ) {
        // let fraction_multiplier = 10_u64.pow(token_mint_details.decimals as u32);

        // Add whole tokens directly
        self.current_balance += amount;

        // let total_fractions = self.fractions + fractions;

        // if total_fractions >= fraction_multiplier {
        //     let overflow_tokens = total_fractions / fraction_multiplier;
        //     let remaining_fractions = total_fractions % fraction_multiplier;

        //     // Add overflowed tokens to current balance
        //     self.current_balance += overflow_tokens;
        //     self.fractions = remaining_fractions;
        // } else {
        //     // No overflow, just update fractions
        //     self.fractions = total_fractions;
        // }
    }

    /// Decrease the balance by the given `amount` and `fractions`.
    pub fn decrease_balance(
        &mut self,
        amount: u64,
        token_mint_details: &TokenMintDetails,
    ) -> Result<(), ProgramError> {
        // Check if sufficient whole tokens are available
        if self.current_balance < amount {
            msg!(
                "Insufficient balance. You have {} tokens but you are trying to spend {}",
                self.current_balance,
                amount
            );
            return Err(ProgramError::InsufficientFunds);
        }

        self.current_balance -= amount;

        Ok(())
    }
}

pub fn initialize_balance_account(
    owner_account: &AccountInfo<'_>,
    mint_account: &AccountInfo<'_>,
    balance_account: &AccountInfo<'_>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !mint_account.is_writable {
        return Err(ProgramError::Immutable);
    }

    if mint_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    if !balance_account.data_is_empty() || balance_account.is_executable {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if balance_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let token_balance =
        TokenBalance::new(owner_account.key.serialize(), mint_account.key.serialize());

    let serialized_token_balance =
        borsh::to_vec(&token_balance).map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    balance_account.realloc(serialized_token_balance.len(), true)?;

    msg!("Changing account data to {:?}!", token_balance);

    balance_account
        .data
        .try_borrow_mut()
        .unwrap()
        .copy_from_slice(&serialized_token_balance);
    Ok(())
}

//cargo test --features=no-entrypoint
#[cfg(test)]
mod balance_change_tests {
    use crate::mint::{InitializeMintInput, MintStatus};

    use super::*;
    use std::collections::HashMap;

    fn create_token_mint_details(mint_price: u64, decimals: u8) -> TokenMintDetails {
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), decimals);
        let token_metadata = HashMap::new();
        TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata)
    }

    #[test]
    fn test_increase_balance_no_fractional_overflow() {
        let mut balance = TokenBalance::new([0u8; 32], [0u8; 32]);
        let mint_details = create_token_mint_details(100, 2); // 2 decimals (100 fractions per token)

        balance.increase_balance(5, &mint_details); // Add 5 tokens
        assert_eq!(balance.current_balance, 5);
    }

    #[test]
    fn test_decrease_balance_no_fractional_underflow() {
        let mut balance = TokenBalance::new([0u8; 32], [0u8; 32]);
        let mint_details = create_token_mint_details(100, 2); // 2 decimals (100 fractions per token)

        balance.increase_balance(5, &mint_details); // Start with 5 tokens
        let result = balance.decrease_balance(3, &mint_details); // Subtract 3 tokens
        assert!(result.is_ok());
        assert_eq!(balance.current_balance, 2);
    }

    #[test]
    fn test_decrease_balance_insufficient_balance() {
        let mut balance = TokenBalance::new([0u8; 32], [0u8; 32]);
        let mint_details = create_token_mint_details(100, 2); // 2 decimals (100 fractions per token)

        balance.increase_balance(2, &mint_details); // Start with 2 tokens
        let result = balance.decrease_balance(3, &mint_details); // Attempt to subtract more than available
        assert!(result.is_err());
    }
}