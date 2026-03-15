use cosmwasm_std::{Event, Reply, StdError, StdResult};
pub use cw_utils::{MsgInstantiateContractResponse, ParseReplyError};

/// Compatibility shim: extracts the contract address (and optional data)
/// from a `Reply` after an `Instantiate` sub-message.
///
/// Replaces the removed `cw_utils::parse_reply_instantiate_data` for
/// cosmwasm-std v2 where `Reply` gained `msg_responses`.
pub fn parse_reply_instantiate_data(
    msg: Reply,
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    let response = msg
        .result
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?;

    // CosmWasm 2.0+: prefer msg_responses
    if let Some(msg_response) = response.msg_responses.first() {
        return cw_utils::parse_instantiate_response_data(&msg_response.value);
    }

    // Fallback for older chains: use deprecated data field
    #[allow(deprecated)]
    let data = response
        .data
        .ok_or_else(|| ParseReplyError::ParseFailure("No data in reply".to_string()))?;

    cw_utils::parse_instantiate_response_data(&data)
}

pub fn parse_event_from_reply_submsg(
    events: Vec<Event>,
    ty: &str,
    attr_key: &str,
) -> StdResult<String> {
    Ok(events
        .iter()
        .find(|e| e.ty == ty) // "instantiate"
        .and_then(|ev| ev.attributes.iter().find(|a| a.key == attr_key)) // "contract_address"
        .or_else(|| {
            events
                .iter()
                .find(|e| e.ty == "wasm")
                .and_then(|ev| ev.attributes.iter().find(|a| a.key == "contract"))
        })
        .ok_or_else(|| StdError::generic_err("key not found in event"))?
        .value
        .clone())
}
