use std::{convert::TryInto, ops::Div};

use cosmwasm_std::{Uint256, Uint512};

/// Computes the amount to add to an address' staked balance when
/// staking.
///
/// # Arguments
///
/// * `staked_total` - The number of tokens that have been staked.
/// * `balance` - The number of tokens the contract has (staked_total + rewards).
/// * `sent` - The number of tokens the user has sent to be staked.
pub(crate) fn amount_to_stake(staked_total: Uint256, balance: Uint256, sent: Uint256) -> Uint256 {
    if staked_total.is_zero() || balance.is_zero() {
        sent
    } else {
        staked_total
            .full_mul(sent)
            .div(Uint512::from(balance))
            .try_into()
            .unwrap() // balance := staked_total + rewards
                      // => balance >= staked_total
                      // => staked_total / balance <= 1
                      // => staked_total * sent / balance <= sent
                      // => we can safely unwrap here as sent fits into a u128 by construction.
    }
}

/// Computes the number of tokens to return to an address when
/// claiming.
///
/// # Arguments
///
/// * `staked_total` - The number of tokens that have been staked.
/// * `balance` - The number of tokens the contract has (staked_total + rewards).
/// * `ask` - The number of tokens being claimed.
///
/// # Invariants
///
/// These must be checked by the caller. If checked, this function is
/// guarenteed not to panic.
///
/// 1. staked_total != 0.
/// 2. ask + balance <= 2^256 (guaranteed since cw20 max supply is 2^128)
/// 3. ask <= staked_total
///
/// All values are `Uint256` to match the cw20 balance field type in
/// cosmwasm-std v3, though cw20 token amounts remain bounded by 2^128.
/// Arithmetic uses `Uint512` intermediates (via `full_mul`) to avoid
/// overflow before the final division.
pub(crate) fn amount_to_claim(staked_total: Uint256, balance: Uint256, ask: Uint256) -> Uint256 {
    // we know that:
    //
    // 1. cw20's max supply is 2^128; values are Uint256 but bounded by 2^128
    // 2. balance := staked_total + rewards
    //
    // for non-malicious inputs:
    //
    // 3. ask <= staked_total  => ask / staked_total <= 1
    // 4. balance <= 2^128
    // 5. 3 + 4 => ask / staked_total * balance <= 2^128 <= Uint256::MAX
    //
    // full_mul returns Uint512 to avoid intermediate overflow, then
    // the result is divided and safely converted back to Uint256.
    ask.full_mul(balance).div(Uint512::from(staked_total)).try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_to_stake_no_overflow() {
        let sent = Uint256::new(2);
        let balance = Uint256::MAX - sent;

        let overflows_naively = sent.checked_mul(balance).is_err();
        assert!(overflows_naively);

        // will panic and fail the test if we've done this wrong.
        amount_to_stake(balance, balance, sent);
    }

    #[test]
    fn test_amount_to_stake_with_zeros() {
        let sent = Uint256::new(42);
        let balance = Uint256::zero();
        let amount = amount_to_stake(balance, balance, sent);
        assert_eq!(amount, sent);
    }

    #[test]
    fn test_amount_to_claim_no_overflow() {
        let ask = Uint256::new(2);
        let balance = Uint256::MAX - ask;

        let overflows_naively = ask.checked_mul(balance).is_err();
        assert!(overflows_naively);

        amount_to_claim(balance, balance, ask);
    }

    // check that our invariants are indeed invariants.

    #[test]
    #[should_panic]
    fn test_amount_to_claim_invariant_one() {
        let ask = Uint256::new(2);
        let balance = Uint256::zero();

        amount_to_claim(balance, balance, ask);
    }

    #[test]
    #[should_panic(expected = "ConversionOverflowError")]
    fn test_amount_to_claim_invariant_two() {
        // Could end up in a situation like this if there are a lot of
        // rewards, but very few staked tokens.
        let ask = Uint256::new(2);
        let balance = Uint256::MAX;
        let staked_total = Uint256::new(1);

        amount_to_claim(staked_total, balance, ask);
    }

    #[test]
    #[should_panic(expected = "ConversionOverflowError")]
    fn test_amount_to_claim_invariant_three() {
        let ask = Uint256::new(2);
        let balance = Uint256::MAX;
        let staked_total = Uint256::new(1);

        amount_to_claim(staked_total, balance, ask);
    }
}
