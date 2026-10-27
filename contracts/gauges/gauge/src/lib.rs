pub mod contract;
mod error;
pub mod msg;
#[cfg(test)]
mod multitest;
pub mod state;

pub use crate::error::ContractError;

/// Seconds since the Unix epoch, truncating nanoseconds.
///
/// Same operation as [`cosmwasm_std::Timestamp::seconds`]: `nanos / 1_000_000_000`.
/// That is an `i64.div_u`. Wasmer singlepass 7.4.2 (libwasmvm v4.0.0-zk)
/// lowers only this division with a `cbz` to the function's div-by-zero trap,
/// and that trap is placed after the body. Dynasm accepts the branch only when
/// the machine-code distance is in [-1048576, 1048572]. Past that, store fails
/// with `ImpossibleRelocation(Dynamic(DynamicLabel(0)))`.
///
/// `instantiate`, `execute`, and `migrate` all call this. `wasm-opt -Oz`
/// inlines a function with one caller, which would put the division back
/// in front of the rest of `execute`. The checker is
/// `terp-rs/scripts/wasm_fn_budget.py`. It rejects a Wasm tail longer than
/// 28062 bytes, the longest tail `terp-core:local-arm64` has stored.
#[inline(never)]
pub fn gauge_block_seconds(nanos: u64) -> u64 {
    nanos / 1_000_000_000
}
