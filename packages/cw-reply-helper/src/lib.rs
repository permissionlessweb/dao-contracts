use cosmwasm_std::{Binary, Event, Reply, StdError, StdResult};
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

/// Extracts the data bytes from a `Reply` after an `Execute` sub-message.
///
/// In cosmwasm-std v2, `msg_responses[0].value` contains a protobuf-encoded
/// `MsgExecuteContractResponse` whose single field is the `data` bytes set by
/// the callee's `Response::set_data`. This function decodes that protobuf
/// wrapper and returns the inner data, falling back to the deprecated
/// `SubMsgResponse::data` field for compatibility with older runtimes and
/// cw-multi-test versions.
pub fn parse_reply_execute_data(msg: &Reply) -> Result<Option<Binary>, ParseReplyError> {
    let response = match &msg.result {
        cosmwasm_std::SubMsgResult::Ok(res) => res,
        cosmwasm_std::SubMsgResult::Err(e) => {
            return Err(ParseReplyError::SubMsgFailure(e.clone()))
        }
    };

    // CosmWasm 2.0+: prefer msg_responses
    if let Some(msg_response) = response.msg_responses.first() {
        let data = parse_msg_execute_contract_response(&msg_response.value)
            .map_err(ParseReplyError::ParseFailure)?;
        return Ok(data.map(Binary::from));
    }

    // Fallback: use deprecated data field
    #[allow(deprecated)]
    Ok(response.data.clone())
}

/// Decode the `data` field from a protobuf-encoded `MsgExecuteContractResponse`.
///
/// The protobuf schema is:
/// ```protobuf
/// message MsgExecuteContractResponse {
///   bytes data = 1;
/// }
/// ```
fn parse_msg_execute_contract_response(raw: &[u8]) -> Result<Option<Vec<u8>>, String> {
    // Empty message means no data was set.
    if raw.is_empty() {
        return Ok(None);
    }

    let mut pos = 0;
    while pos < raw.len() {
        let (tag, new_pos) =
            decode_varint(raw, pos).map_err(|e| format!("failed to decode tag: {e}"))?;
        pos = new_pos;

        let field_number = tag >> 3;
        let wire_type = tag & 0x7;

        if wire_type != 2 {
            return Err(format!("unexpected wire type {wire_type} for field {field_number}"));
        }

        let (len, new_pos) =
            decode_varint(raw, pos).map_err(|e| format!("failed to decode length: {e}"))?;
        pos = new_pos;
        let len = len as usize;

        if pos + len > raw.len() {
            return Err("truncated protobuf data".to_string());
        }

        if field_number == 1 {
            return Ok(Some(raw[pos..pos + len].to_vec()));
        }

        // Skip unknown fields
        pos += len;
    }

    // Field 1 not present — no data
    Ok(None)
}

/// Minimal varint decoder for protobuf tag/length fields.
fn decode_varint(buf: &[u8], mut pos: usize) -> Result<(u64, usize), &'static str> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if pos >= buf.len() {
            return Err("unexpected end of varint");
        }
        let byte = buf[pos];
        pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, pos));
        }
        shift += 7;
        if shift >= 64 {
            return Err("varint too long");
        }
    }
}

pub fn parse_event_from_reply_submsg(
    events: Vec<Event>,
    ty: &str,
    attr_key: &str,
) -> StdResult<String> {
    // cosmwasm-std v2+ prefixes system attributes with underscore
    let prefixed_key = format!("_{attr_key}");
    Ok(events
        .iter()
        .find(|e| e.ty == ty)
        .and_then(|ev| {
            ev.attributes
                .iter()
                .find(|a| a.key == attr_key || a.key == prefixed_key)
        })
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
