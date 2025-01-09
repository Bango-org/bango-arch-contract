use arch_program::program_error::ProgramError;

use crate::mint::{MintInput, TokenMintDetails};

pub fn calculate_mint_price_in_sats(
    mint_details: &TokenMintDetails,
    mint_input: &MintInput,
) -> Result<u64, ProgramError> {
    let base_price = mint_input.amount * mint_details.mint_price_in_sats;

    // Calculate the fractional price with ceiling rounding
    let fraction_multiplier = 10_u64.pow(mint_details.decimals as u32);
    let fractional_price =
        (mint_details.mint_price_in_sats * mint_input.fractions + fraction_multiplier - 1)
            / fraction_multiplier;

    // Total price is the sum of base price and fractional price
    Ok(base_price + fractional_price)
}

// Need to use feature :: cargo test --features no-entrypoint
#[cfg(test)]
mod calculate_mint_price_tests {
    use crate::mint::{InitializeMintInput, MintStatus};

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_calculate_price_no_fraction() {
        // Test with no fractional part
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 100, 2);
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(10, 0, vec![]);

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 1000); // 10 tokens * 100 sats each
    }

    #[test]
    fn test_calculate_price_with_fraction_rounding_up() {
        // Test with a fractional part that requires rounding up
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 100, 2);
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(10, 25, vec![]); // 25 out of 100 (0.25 of a token)

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 1025); // 1000 + 25 (fraction rounded up)
    }

    #[test]
    fn test_calculate_price_full_fraction() {
        // Test with a full fractional part equal to 1 token
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 100, 2);
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(10, 100, vec![]); // 100 out of 100 (1 full token)

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 1100); // 1000 + 100 (1 extra token's worth)
    }

    #[test]
    fn test_calculate_price_decimals_more_than_1_token() {
        // Test with a full fractional part equal to 1 token
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 100, 2);
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(10, 200, vec![]); // 100 out of 100 (1 full token)

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 1200); // 1000 + 100 (1 extra token's worth)
    }

    #[test]
    fn test_calculate_price_high_decimals_precision() {
        // Test with high decimals (0.75 of a token)
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 100, 2);
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(10, 75, vec![]); // 75 out of 100 (0.75 of a token)

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 1075); // 1000 + 75
    }

    #[test]
    fn test_calculate_price_zero_amount() {
        // Test with zero amount, should only use fractional price
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 100, 2);
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(0, 50, vec![]); // 0 tokens, only 0.5 token as fraction

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 50); // Only the fractional price (50 out of 100)
    }

    #[test]
    fn test_calculate_price_fraction_less_than_one_sat() {
        // Test with a fractional price that should round to 0 (less than 1 sat)
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 1, 8); // 1 sat mint price, 8 decimals
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(10, 1, vec![]); // Small fraction: 1 out of 10^8

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 11); // Fraction is less than 1 sat, so should round up
    }

    #[test]
    fn test_calculate_price_very_small_fraction() {
        // Test with an even smaller fractional cost (decimal part much smaller than 1 sat)
        let owner = [0u8; 32];
        let initialize_input = InitializeMintInput::new(owner, 1000, "TEST".to_string(), 10, 8); // 10 sats mint price, 8 decimals
        let token_metadata = HashMap::new();
        let mint_details =
            TokenMintDetails::new(initialize_input, MintStatus::Ongoing, token_metadata, [0;32]);

        let mint_input = MintInput::new(10, 1, vec![]); // 1 out of 10^8, effectively zero sats

        let price = calculate_mint_price_in_sats(&mint_details, &mint_input).unwrap();

        assert_eq!(price, 101); // Fraction is less than 1 sat, so should round up 1
    }
}