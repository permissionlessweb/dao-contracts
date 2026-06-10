 
 
fn main() -> cw_orch::anyhow::Result<()> {
    // rustls::crypto::aws_lc_rs::default_provider()
    //     .install_default()
    //     .unwrap();

    // dotenv::dotenv().ok();
    deploy_dao()
}

/// Derive full ibc state for a given chain specified in the chain registry format:
/// - ibc_data schema defined client, channel, connection, port information
/// - asset_list: IBC denom hashes for foreign tokens defined in state.json assets list
fn deploy_dao() -> cw_orch::anyhow::Result<()> {

    Ok(())
}
