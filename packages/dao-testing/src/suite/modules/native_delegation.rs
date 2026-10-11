//! Native staking + bank actions a DAO executes from its treasury.
//!
//! Complements `DaoVoteDelegation` (voting-power delegates). This module
//! builds CosmosMsgs for `MsgDelegate` / `MsgSend` so Foundation-style
//! proposals (spread stake, gas a new operator, instantiate a subDAO) share
//! one builder.

use cosmwasm_std::{Coin, CosmosMsg, Uint256, WasmMsg};

/// 6-decimal micro units (uterp / uthiol).
pub const MICRO: u128 = 1_000_000;

/// Live morocco-1 operator account (quoted). Operator bech32 is derived.
pub const NEW_VALIDATOR_ACCOUNT: &str = "terp1gr67fce5ymhyz93qe0flqlgv2p424a48x8atl8";
/// Derived `terpvaloper` for [`NEW_VALIDATOR_ACCOUNT`] (same words, valoper HRP).
pub const NEW_VALIDATOR_OPERATOR: &str = "terpvaloper1gr67fce5ymhyz93qe0flqlgv2p424a48hccllk";

pub const STAKE_DENOM: &str = "uterp";
pub const GAS_DENOM: &str = "uthiol";

/// Parent proposal copy (review before any live submit).
pub const FOUNDATION_TITLE: &str = "Say fuck it and begin";
pub const FOUNDATION_DESCRIPTION: &str = "\
FIAB-SUBDAO is ours. Fuck it and begin — explore and use the environment without \
apology, then look at what you did.

This vote stands FIAB up under this DAO. We stay admin. Seed 1,000,000 uterp and \
1,000,000 uthiol.

It also delegates 100,000 TERP to every bonded validator.";
pub const FIAB_NAME: &str = "FIAB-SUBDAO";
pub const FIAB_DESCRIPTION: &str = "Fuck it and begin. Explore and use the environment without apology. Shape it through action and reflection.";

/// 200_000 TERP in uterp.
pub fn terp_200k() -> Uint256 {
    Uint256::from(200_000u128 * MICRO)
}

/// 100_000 TERP in uterp.
pub fn terp_100k() -> Uint256 {
    Uint256::from(100_000u128 * MICRO)
}

/// 7.10 THIOL in uthiol.
pub fn thiol_7_10() -> Uint256 {
    Uint256::from(7_100_000u128)
}

pub fn coin(amount: Uint256, denom: &str) -> Coin {
    Coin {
        denom: denom.to_string(),
        amount,
    }
}

/// `CosmosMsg::Staking(Delegate)` — delegator is the executing DAO.
pub fn delegate_msg(validator: impl Into<String>, amount: Uint256, denom: &str) -> CosmosMsg {
    CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
        validator: validator.into(),
        amount: coin(amount, denom),
    })
}

/// `CosmosMsg::Bank(Send)` from the DAO treasury.
pub fn bank_send_msg(to: impl Into<String>, amount: Uint256, denom: &str) -> CosmosMsg {
    CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: to.into(),
        amount: vec![coin(amount, denom)],
    })
}

/// Re-encode a bech32 account as `terpvaloper…`. No-op if already valoper.
pub fn to_valoper(addr: &str) -> Result<String, String> {
    if addr.starts_with("terpvaloper1") {
        return Ok(addr.to_string());
    }
    let Some((_, rest)) = addr.split_once('1') else {
        return Err(format!("not bech32: {addr}"));
    };
    // Keep payload+checksum; swap HRP. Caller should prefer a quoted valoper.
    // Full convert: decode words minus checksum, re-checksum under new HRP.
    convert_hrp(addr, "terpvaloper")
}

fn convert_hrp(addr: &str, new_hrp: &str) -> Result<String, String> {
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let (hrp, data) = addr.split_once('1').ok_or_else(|| format!("not bech32: {addr}"))?;
    let _ = hrp;
    let vals: Result<Vec<u8>, _> = data
        .bytes()
        .map(|c| {
            CHARSET
                .iter()
                .position(|&x| x == c)
                .map(|i| i as u8)
                .ok_or("bad bech32 char")
        })
        .collect();
    let mut vals = vals.map_err(|e| e.to_string())?;
    if vals.len() < 6 {
        return Err("bech32 too short".into());
    }
    vals.truncate(vals.len() - 6);
    Ok(bech32_encode(new_hrp, &vals))
}

fn bech32_polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut chk: u32 = 1;
    for v in values {
        let b = chk >> 25;
        chk = ((chk & 0x1ffffff) << 5) ^ (*v as u32);
        for i in 0..5 {
            if (b >> i) & 1 == 1 {
                chk ^= GEN[i];
            }
        }
    }
    chk
}

fn bech32_encode(hrp: &str, data: &[u8]) -> String {
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let mut values: Vec<u8> = hrp.bytes().map(|x| x >> 5).collect();
    values.push(0);
    values.extend(hrp.bytes().map(|x| x & 31));
    values.extend_from_slice(data);
    values.extend_from_slice(&[0; 6]);
    let polymod = bech32_polymod(&values) ^ 1;
    let mut checksum = [0u8; 6];
    for i in 0..6 {
        checksum[i] = ((polymod >> (5 * (5 - i))) & 31) as u8;
    }
    let mut out = String::from(hrp);
    out.push('1');
    for v in data.iter().chain(checksum.iter()) {
        out.push(CHARSET[*v as usize] as char);
    }
    out
}

/// One Foundation-style native delegation campaign.
#[derive(Clone, Debug)]
pub struct NativeDelegationPlan {
    pub stake_denom: String,
    pub gas_denom: String,
    pub new_validator_operator: String,
    pub new_validator_account: String,
    pub new_validator_stake: Uint256,
    pub new_validator_gas: Uint256,
    pub other_validators: Vec<String>,
    pub other_stake: Uint256,
}

impl NativeDelegationPlan {
    /// morocco-1 defaults: 200k TERP + 7.10 THIOL to the new operator,
    /// 100k TERP to every other bonded validator.
    pub fn terp_foundation(other_validators: Vec<String>) -> Self {
        let others: Vec<String> = other_validators
            .into_iter()
            .filter(|v| v != NEW_VALIDATOR_OPERATOR && v != NEW_VALIDATOR_ACCOUNT)
            .collect();
        Self {
            stake_denom: STAKE_DENOM.into(),
            gas_denom: GAS_DENOM.into(),
            new_validator_operator: NEW_VALIDATOR_OPERATOR.into(),
            new_validator_account: NEW_VALIDATOR_ACCOUNT.into(),
            new_validator_stake: terp_200k(),
            new_validator_gas: thiol_7_10(),
            other_validators: others,
            other_stake: terp_100k(),
        }
    }

    /// 100k TERP to each bonded validator. No new-operator stake or THIOL.
    pub fn bonded_set(other_validators: Vec<String>) -> Self {
        let mut plan = Self::terp_foundation(other_validators);
        plan.new_validator_stake = Uint256::zero();
        plan.new_validator_gas = Uint256::zero();
        plan
    }

    pub fn cosmos_msgs(&self) -> Vec<CosmosMsg> {
        let mut msgs = Vec::with_capacity(self.other_validators.len() + 2);
        if !self.new_validator_stake.is_zero() {
            msgs.push(delegate_msg(
                &self.new_validator_operator,
                self.new_validator_stake,
                &self.stake_denom,
            ));
        }
        if !self.new_validator_gas.is_zero() {
            msgs.push(bank_send_msg(
                &self.new_validator_account,
                self.new_validator_gas,
                &self.gas_denom,
            ));
        }
        for v in &self.other_validators {
            msgs.push(delegate_msg(v, self.other_stake, &self.stake_denom));
        }
        msgs
    }

    /// Append extra DAO actions (e.g. FIAB-SUBDAO instantiate) so one proposal
    /// can stake and spin a subDAO together.
    pub fn with_extra(self, extra: Vec<CosmosMsg>) -> Vec<CosmosMsg> {
        let mut msgs = self.cosmos_msgs();
        msgs.extend(extra);
        msgs
    }
}

/// Wasm instantiate of a subDAO under `admin` (parent DAO). Funds seed the
/// new core. `msg` is the dao-dao-core InstantiateMsg JSON bytes.
pub fn instantiate_subdao_msg(
    admin: impl Into<String>,
    code_id: u64,
    label: impl Into<String>,
    msg: Vec<u8>,
    funds: Vec<Coin>,
) -> CosmosMsg {
    let admin = admin.into();
    CosmosMsg::Wasm(WasmMsg::Instantiate {
        admin: Some(admin),
        code_id,
        msg: msg.into(),
        funds,
        label: label.into(),
    })
}

/// Vote-yes payload for dao-proposal-single (after the proposal exists).
pub fn vote_yes_msg(proposal_id: u64) -> dao_proposal_single::msg::ExecuteMsg {
    dao_proposal_single::msg::ExecuteMsg::Vote {
        proposal_id,
        vote: dao_voting::voting::Vote::Yes,
        rationale: None,
    }
}

pub fn execute_proposal_msg(proposal_id: u64) -> dao_proposal_single::msg::ExecuteMsg {
    dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valoper_convert_matches_quoted() {
        assert_eq!(
            to_valoper(NEW_VALIDATOR_ACCOUNT).unwrap(),
            NEW_VALIDATOR_OPERATOR
        );
        assert_eq!(
            to_valoper(NEW_VALIDATOR_OPERATOR).unwrap(),
            NEW_VALIDATOR_OPERATOR
        );
    }

    #[test]
    fn plan_excludes_new_from_others_and_uses_weight_amounts() {
        let others = vec![
            NEW_VALIDATOR_OPERATOR.to_string(),
            "terpvaloper1aaaa".into(),
            "terpvaloper1bbbb".into(),
        ];
        let plan = NativeDelegationPlan::terp_foundation(others);
        assert_eq!(plan.other_validators.len(), 2);
        let msgs = plan.cosmos_msgs();
        // 1 delegate new + 1 send + 2 others
        assert_eq!(msgs.len(), 4);
        match &msgs[0] {
            CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate { validator, amount }) => {
                assert_eq!(validator, NEW_VALIDATOR_OPERATOR);
                assert_eq!(amount.amount, terp_200k());
                assert_eq!(amount.denom, "uterp");
            }
            other => panic!("{other:?}"),
        }
        match &msgs[1] {
            CosmosMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, NEW_VALIDATOR_ACCOUNT);
                assert_eq!(amount[0].amount, thiol_7_10());
                assert_eq!(amount[0].denom, "uthiol");
            }
            other => panic!("{other:?}"),
        }
        match &msgs[2] {
            CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate { amount, .. }) => {
                assert_eq!(amount.amount, terp_100k());
            }
            other => panic!("{other:?}"),
        }
    }
}
