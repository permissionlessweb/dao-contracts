use cw_orch::{anyhow, prelude::*};
use dao_testing::DaoDaoSuite;

fn main() -> anyhow::Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();
    env_logger::init();
    dotenv::dotenv().ok();
    let terp = Daemon::builder(networks::TERP_MAINNET).build()?;
    
    let dao = DaoDaoSuite::new(terp.clone());
    // dao.voting.cw4_group.upload()?;
    dao.staking.upload()?;
    dao.distribution.upload()?;
    dao.external.upload()?;
    dao.gauges.upload()?;

    let id = terp.state().get_all_code_ids()?;
    println!("{:#?}", id);

    Ok(())
}
