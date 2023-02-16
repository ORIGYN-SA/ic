use candid::Principal;
use dfn_candid::candid_one;
use ic_sns_cli::init_config_file::{
    SnsCliInitConfig, SnsGovernanceConfig, SnsInitialTokenDistributionConfig, SnsLedgerConfig,
};
use ic_sns_governance::pb::v1::{
    GetSnsInitializationParametersRequest, GetSnsInitializationParametersResponse,
};
use ic_sns_governance::types::ONE_MONTH_SECONDS;
use ic_sns_init::pb::v1::sns_init_payload::InitialTokenDistribution::FractionalDeveloperVotingPower;
use ic_sns_init::pb::v1::{FractionalDeveloperVotingPower as FractionalDVP, SnsInitPayload};
use ic_sns_init::SnsCanisterIds;
use ic_sns_test_utils::itest_helpers::{local_test_on_sns_subnet, SnsCanisters};
use ic_types::PrincipalId;
use std::convert::TryFrom;

fn get_test_sns_cli_init_config() -> SnsCliInitConfig {
    SnsCliInitConfig {
        sns_ledger: SnsLedgerConfig {
            transaction_fee_e8s: Some(10_000),
            token_name: Some("ServiceNervousSystem".to_string()),
            token_symbol: Some("SNS".to_string()),
        },
        sns_governance: SnsGovernanceConfig {
            proposal_reject_cost_e8s: Some(100_000_000),
            neuron_minimum_stake_e8s: Some(100_000_000),
            fallback_controller_principal_ids: vec![Principal::from(
                PrincipalId::new_user_test_id(1_552_301),
            )
            .to_text()],
            logo: None,
            name: Some("ServiceNervousSystem".to_string()),
            description: Some("A project that decentralizes a dapp".to_string()),
            url: Some("https://internetcomputer.org/".to_string()),
            neuron_minimum_dissolve_delay_to_vote_seconds: Some(0),
            initial_reward_rate_percentage: Some(31.0),
            final_reward_rate_percentage: Some(21.0),
            reward_rate_transition_duration_seconds: Some(100_000),
            max_dissolve_delay_seconds: Some(8 * ONE_MONTH_SECONDS),
            max_neuron_age_seconds_for_age_bonus: Some(11 * ONE_MONTH_SECONDS),
            max_dissolve_delay_bonus_multiplier: Some(1.3),
            max_age_bonus_multiplier: Some(1.8),
            initial_voting_period_seconds: Some(1006700),
            wait_for_quiet_deadline_increase_seconds: Some(86700),
        },
        initial_token_distribution: SnsInitialTokenDistributionConfig {
            initial_token_distribution: Some(FractionalDeveloperVotingPower(
                FractionalDVP::with_valid_values_for_testing(),
            )),
        },
    }
}
#[test]
fn test_get_init_config_file() {
    local_test_on_sns_subnet(|runtime| async move {
        let sns_init_payload = SnsInitPayload::try_from(get_test_sns_cli_init_config())
            .expect("Error: couldn't convert SnsCliInitConfig into SnsInitPayload");

        let sns_canisters_init_payload = sns_init_payload
            .build_canister_payloads(
                &SnsCanisterIds {
                    governance: PrincipalId::new_user_test_id(1),
                    ledger: PrincipalId::new_user_test_id(2),
                    root: PrincipalId::new_user_test_id(3),
                    swap: PrincipalId::new_user_test_id(4),
                    index: PrincipalId::new_user_test_id(5),
                },
                None,
                false,
            )
            .unwrap();
        let sns_canisters = SnsCanisters::set_up(&runtime, sns_canisters_init_payload).await;

        let get_sns_initialization_parameters_response: GetSnsInitializationParametersResponse =
            sns_canisters
                .governance
                .query_(
                    "get_sns_initialization_parameters",
                    candid_one,
                    GetSnsInitializationParametersRequest {},
                )
                .await
                .expect("Error calling get_sns_initialization_parameters api");

        let expected_initialization_parameters = serde_yaml::to_string(&sns_init_payload).unwrap();

        assert_eq!(
            get_sns_initialization_parameters_response.sns_initialization_parameters,
            expected_initialization_parameters
        );

        Ok(())
    });
}
