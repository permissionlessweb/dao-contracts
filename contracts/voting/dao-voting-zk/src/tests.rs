#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, MockApi, MockStorage},
        Empty, MessageInfo, OwnedDeps, Uint256,
    };

    use crate::contract::{execute, instantiate, query};
    use crate::error::ContractError;
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SnapshotResponse};
    use crate::state::{DAO, POLL_REGISTRY, SNAPSHOTS, UNDERLYING_VOTING_MODULE};

    fn addr_make(prefix: &str) -> cosmwasm_std::Addr {
        MockApi::default().addr_make(prefix)
    }

    fn mock_info(sender: &str) -> MessageInfo {
        MessageInfo {
            sender: addr_make(sender),
            funds: vec![],
        }
    }

    fn setup_test() -> OwnedDeps<MockStorage, MockApi, cosmwasm_std::testing::MockQuerier, Empty>
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("dao");

        let msg = InstantiateMsg {
            underlying_voting_module: addr_make("underlying_module").to_string(),
            poll_registry: addr_make("poll_registry").to_string(),
            active_threshold: None,
        };

        instantiate(deps.as_mut(), env, info, msg).unwrap();
        deps
    }

    #[test]
    fn test_instantiate() {
        let deps = setup_test();

        let dao = DAO.load(deps.as_ref().storage).unwrap();
        assert_eq!(dao, addr_make("dao"));

        let underlying = UNDERLYING_VOTING_MODULE.load(deps.as_ref().storage).unwrap();
        assert_eq!(underlying, addr_make("underlying_module"));

        let registry = POLL_REGISTRY.load(deps.as_ref().storage).unwrap();
        assert_eq!(registry, addr_make("poll_registry"));
    }

    #[test]
    fn test_unauthorized_update_config() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("alice");

        let err = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::UpdateConfig {
                underlying_voting_module: None,
                poll_registry: None,
            },
        )
        .unwrap_err();

        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_unauthorized_snapshot() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("alice");

        let err = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::SnapshotVoters {
                proposal_id: 1,
                merkle_root: "0xabcd".to_string(),
                total_power: Uint256::from(100u128),
            },
        )
        .unwrap_err();

        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_snapshot_duplicate_proposal() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("dao");

        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::SnapshotVoters {
                proposal_id: 1,
                merkle_root: "0xabcd".to_string(),
                total_power: Uint256::from(100u128),
            },
        )
        .unwrap();

        let err = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::SnapshotVoters {
                proposal_id: 1,
                merkle_root: "0xef01".to_string(),
                total_power: Uint256::from(200u128),
            },
        )
        .unwrap_err();

        assert_eq!(err, ContractError::SnapshotAlreadyExists { proposal_id: 1 });
    }

    #[test]
    fn test_snapshot_zero_total_power() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("dao");

        let err = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::SnapshotVoters {
                proposal_id: 1,
                merkle_root: "0xabcd".to_string(),
                total_power: Uint256::zero(),
            },
        )
        .unwrap_err();

        assert_eq!(err, ContractError::ZeroTotalPower {});
    }

    #[test]
    fn test_snapshot_empty_merkle_root() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("dao");

        let err = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::SnapshotVoters {
                proposal_id: 1,
                merkle_root: "".to_string(),
                total_power: Uint256::from(100u128),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::InvalidMerkleRoot {
                reason: "empty merkle root".to_string()
            }
        );
    }

    #[test]
    fn test_snapshot_success() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("dao");

        execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::SnapshotVoters {
                proposal_id: 42,
                merkle_root: "0xdeadbeef".to_string(),
                total_power: Uint256::from(1000u128),
            },
        )
        .unwrap();

        let snap = SNAPSHOTS.load(deps.as_ref().storage, 42).unwrap();
        assert_eq!(snap.proposal_id, 42);
        assert_eq!(snap.merkle_root, "0xdeadbeef");
        assert_eq!(snap.total_power, Uint256::from(1000u128));
        assert!(snap.poll_id.starts_with("zk_vote_42_"));
        assert_eq!(snap.created_at, mock_env().block.height);
    }

    #[test]
    fn test_query_snapshot() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("dao");

        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ExecuteMsg::SnapshotVoters {
                proposal_id: 7,
                merkle_root: "0xcafe".to_string(),
                total_power: Uint256::from(500u128),
            },
        )
        .unwrap();

        let binary = query(deps.as_ref(), env, QueryMsg::Snapshot { proposal_id: 7 }).unwrap();

        let resp: SnapshotResponse = cosmwasm_std::from_json(&binary).unwrap();
        assert_eq!(resp.proposal_id, 7);
        assert_eq!(resp.merkle_root, "0xcafe");
        assert_eq!(resp.total_power, Uint256::from(500u128));
    }

    #[test]
    fn test_update_config() {
        let mut deps = setup_test();
        let env = mock_env();
        let info = mock_info("dao");

        execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::UpdateConfig {
                underlying_voting_module: Some(addr_make("new_underlying").to_string()),
                poll_registry: Some(addr_make("new_registry").to_string()),
            },
        )
        .unwrap();

        let underlying = UNDERLYING_VOTING_MODULE.load(deps.as_ref().storage).unwrap();
        assert_eq!(underlying, addr_make("new_underlying"));

        let registry = POLL_REGISTRY.load(deps.as_ref().storage).unwrap();
        assert_eq!(registry, addr_make("new_registry"));
    }
}