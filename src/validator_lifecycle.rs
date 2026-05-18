use crate::crypto::aegis_pqvm::AegisPqvmVerifier;
use crate::synergy_types::{
    CanonicalSerialize, ProtocolConfig, QuorumCertificate, StakeStatus, Transaction,
    ValidatorRecord, ValidatorSet, ValidatorStakeRecord, ValidatorStatus,
};
use std::collections::BTreeMap;

pub const REQUIRED_VALIDATOR_STAKE_SNRG: u64 = 50_000;
pub const REQUIRED_VALIDATOR_STAKE_NWEI: u128 = 50_000_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorLifecycleState {
    pub validator: ValidatorRecord,
    pub stake: Option<ValidatorStakeRecord>,
    pub completed_steps: Vec<ValidatorStatus>,
    pub blocking_reason: String,
}

impl ValidatorLifecycleState {
    pub fn new(validator: ValidatorRecord) -> Self {
        Self {
            validator,
            stake: None,
            completed_steps: Vec::new(),
            blocking_reason: String::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ValidatorLifecycleManager {
    states: BTreeMap<String, ValidatorLifecycleState>,
}

impl ValidatorLifecycleManager {
    pub fn insert(&mut self, state: ValidatorLifecycleState) {
        self.states
            .insert(state.validator.validator_id.0.clone(), state);
    }

    pub fn state(&self, validator_id: &str) -> Option<&ValidatorLifecycleState> {
        self.states.get(validator_id)
    }

    pub fn state_mut(&mut self, validator_id: &str) -> Option<&mut ValidatorLifecycleState> {
        self.states.get_mut(validator_id)
    }

    pub fn key_bound_requires_stake(&mut self, validator_id: &str) -> Result<(), String> {
        let state = self
            .state_mut(validator_id)
            .ok_or_else(|| "validator lifecycle state not found".to_string())?;
        if state.validator.status != ValidatorStatus::KeyBound {
            return Err("validator must be KEY_BOUND before STAKE_REQUIRED".to_string());
        }
        state.completed_steps.push(ValidatorStatus::KeyBound);
        state.validator.status = ValidatorStatus::StakeRequired;
        state.blocking_reason = "Stake 50,000 SNRG to continue validator onboarding.".to_string();
        Ok(())
    }

    pub fn submit_stake(
        &mut self,
        validator_id: &str,
        stake: ValidatorStakeRecord,
        stake_tx: &Transaction,
        verifier: &AegisPqvmVerifier,
    ) -> Result<(), String> {
        let state = self
            .state_mut(validator_id)
            .ok_or_else(|| "validator lifecycle state not found".to_string())?;
        if state.validator.status != ValidatorStatus::StakeRequired {
            return Err("stake can only be submitted from STAKE_REQUIRED".to_string());
        }
        stake_tx.chain_id.require_testnet_v2()?;
        stake_tx.network_id.require_testnet_v2()?;
        verifier
            .verify_transaction_signature_checked(stake_tx)
            .map_err(|error| error.to_string())?;
        if stake.stake_amount_nwei < REQUIRED_VALIDATOR_STAKE_NWEI {
            state.blocking_reason = "Submitted stake is below 50,000 SNRG.".to_string();
            return Err("insufficient validator stake".to_string());
        }
        if stake.required_stake_nwei != REQUIRED_VALIDATOR_STAKE_NWEI {
            return Err("stake record required amount does not match protocol minimum".to_string());
        }
        state.stake = Some(stake);
        state.completed_steps.push(ValidatorStatus::StakeRequired);
        state.validator.status = ValidatorStatus::StakeSubmitted;
        state.blocking_reason = "Stake transaction is pending finality.".to_string();
        Ok(())
    }

    pub fn confirm_stake(
        &mut self,
        validator_id: &str,
        finalized_qc: &QuorumCertificate,
        validator_set: &ValidatorSet,
        verifier: &AegisPqvmVerifier,
    ) -> Result<(), String> {
        let state = self
            .state_mut(validator_id)
            .ok_or_else(|| "validator lifecycle state not found".to_string())?;
        if state.validator.status != ValidatorStatus::StakeSubmitted {
            return Err("stake can only be confirmed from STAKE_SUBMITTED".to_string());
        }
        let stake = state
            .stake
            .as_mut()
            .ok_or_else(|| "stake record missing".to_string())?;
        if !verifier.verify_qc(
            finalized_qc,
            validator_set,
            &crate::synergy_types::ClusterMap {
                epoch: validator_set.epoch,
                assignments: validator_set
                    .validators
                    .iter()
                    .map(|record| crate::synergy_types::ClusterAssignment {
                        cluster_id: record.cluster_id,
                        validator_id: record.validator_id.clone(),
                    })
                    .collect(),
            },
        ) {
            stake.stake_status = StakeStatus::InvalidSignature;
            return Err("stake finalized block QC failed Aegis PQC verification".to_string());
        }
        if stake.stake_amount_nwei < REQUIRED_VALIDATOR_STAKE_NWEI {
            stake.stake_status = StakeStatus::Insufficient;
            return Err("stake below required minimum".to_string());
        }
        if !stake.stake_slashable {
            return Err("stake lock is not slashable under validator rules".to_string());
        }
        stake.stake_status = StakeStatus::Locked;
        stake.stake_verified = true;
        stake.stake_finalized_qc_hash = crate::synergy_types::Hash::from_domain_bytes(
            "SYNERGY_STAKE_FINALIZED_QC_V1",
            &finalized_qc.canonical_bytes()?,
        );
        state.completed_steps.push(ValidatorStatus::StakeSubmitted);
        state.validator.status = ValidatorStatus::StakeConfirmed;
        state.blocking_reason.clear();
        Ok(())
    }

    pub fn advance_after_stake(
        &mut self,
        validator_id: &str,
        next_status: ValidatorStatus,
        protocol: &ProtocolConfig,
    ) -> Result<(), String> {
        let state = self
            .state_mut(validator_id)
            .ok_or_else(|| "validator lifecycle state not found".to_string())?;
        if matches!(
            next_status,
            ValidatorStatus::Syncing
                | ValidatorStatus::SnapshotVerified
                | ValidatorStatus::Replaying
                | ValidatorStatus::Shadow
                | ValidatorStatus::Ready
                | ValidatorStatus::PendingActivation
                | ValidatorStatus::Active
        ) {
            let stake = state.stake.as_ref().ok_or_else(|| {
                "confirmed stake is required before onboarding can continue".to_string()
            })?;
            if !stake.satisfies_required_stake(protocol) {
                return Err(
                    "confirmed finalized locked stake is required before onboarding can continue"
                        .to_string(),
                );
            }
        }
        if matches!(
            next_status,
            ValidatorStatus::Ready | ValidatorStatus::PendingActivation | ValidatorStatus::Active
        ) && state.validator.status != ValidatorStatus::StakeConfirmed
            && state.validator.status != ValidatorStatus::Syncing
            && state.validator.status != ValidatorStatus::SnapshotVerified
            && state.validator.status != ValidatorStatus::Replaying
            && state.validator.status != ValidatorStatus::Shadow
        {
            return Err(
                "validator cannot skip onboarding stages after stake confirmation".to_string(),
            );
        }
        state.completed_steps.push(state.validator.status.clone());
        state.validator.status = next_status;
        Ok(())
    }

    pub fn can_vote(&self, validator_id: &str) -> bool {
        self.state(validator_id)
            .map(|state| state.validator.status == ValidatorStatus::Active)
            .unwrap_or(false)
    }

    pub fn can_propose(&self, validator_id: &str) -> bool {
        self.can_vote(validator_id)
    }
}

pub fn lifecycle_order() -> Vec<ValidatorStatus> {
    vec![
        ValidatorStatus::Registered,
        ValidatorStatus::KeyBound,
        ValidatorStatus::StakeRequired,
        ValidatorStatus::StakeSubmitted,
        ValidatorStatus::StakeConfirmed,
        ValidatorStatus::Syncing,
        ValidatorStatus::SnapshotVerified,
        ValidatorStatus::Replaying,
        ValidatorStatus::Shadow,
        ValidatorStatus::Ready,
        ValidatorStatus::PendingActivation,
        ValidatorStatus::Active,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::aegis_pqvm::AegisPqvmSigner;
    use crate::synergy_types::{
        AegisPqKeyId, AegisPqKeyRole, AegisPqSignature, ChainId, ClusterId, Epoch, Hash, Height,
        NetworkId, TxId, UmaId, ValidatorId,
    };

    fn validator_state() -> (AegisPqvmSigner, ValidatorLifecycleState, AegisPqKeyId) {
        let mut signer = AegisPqvmSigner::initialize_required().unwrap();
        let key_id = signer
            .generate_and_register_key("uma-1", vec![AegisPqKeyRole::Transaction], Epoch(0))
            .unwrap();
        let public = signer.public_key_record(&key_id).unwrap();
        let validator = ValidatorRecord {
            validator_id: ValidatorId("validator-1".to_string()),
            validator_uma_id: UmaId("uma-1".to_string()),
            consensus_public_key: public.clone(),
            peer_public_key: public.clone(),
            operator_public_key: public,
            voting_weight: 1,
            status: ValidatorStatus::KeyBound,
            cluster_id: ClusterId(0),
            activation_epoch: Epoch(0),
        };
        (signer, ValidatorLifecycleState::new(validator), key_id)
    }

    fn stake_record(amount: u128) -> ValidatorStakeRecord {
        ValidatorStakeRecord {
            validator_id: ValidatorId("validator-1".to_string()),
            validator_uma_id: UmaId("uma-1".to_string()),
            stake_owner: "uma-1".to_string(),
            stake_amount_nwei: amount,
            required_stake_nwei: REQUIRED_VALIDATOR_STAKE_NWEI,
            stake_tx_hash: TxId("stake-tx".to_string()),
            stake_lock_id: "stake-lock".to_string(),
            stake_status: StakeStatus::Submitted,
            stake_finalized_height: Height(1),
            stake_finalized_block_hash: Hash::zero(),
            stake_finalized_qc_hash: Hash::zero(),
            stake_activation_epoch: Epoch(1),
            stake_unlock_epoch_optional: None,
            stake_slashable: true,
            stake_verified: false,
        }
    }

    fn signed_stake_tx(
        signer: &mut AegisPqvmSigner,
        key_id: &AegisPqKeyId,
        amount: u128,
    ) -> Transaction {
        let mut tx = Transaction {
            version: 1,
            chain_id: ChainId::synergy_testnet_v2(),
            network_id: NetworkId::synergy_testnet_v2(),
            epoch: Epoch(0),
            sender_uma_or_account: "uma-1".to_string(),
            receiver_uma_or_account: "validator-staking".to_string(),
            account_nonce_or_sequence: 0,
            amount_nwei: amount,
            gas_limit: 21_000,
            max_fee_nwei: 1,
            ttl_height: Height(10),
            explicit_dependencies: Vec::new(),
            read_set_hint: Vec::new(),
            write_set_hint: vec!["validator-stake:validator-1".to_string()],
            payload: b"validator-stake".to_vec(),
            signer_uma_id: UmaId("uma-1".to_string()),
            aegis_pq_key_id: key_id.clone(),
            aegis_pq_signature: AegisPqSignature {
                algorithm: String::new(),
                signature_bytes: Vec::new(),
            },
        };
        tx.aegis_pq_signature = signer
            .sign_transaction(&tx.signing_bytes().unwrap(), key_id)
            .unwrap();
        tx
    }

    #[test]
    fn new_validator_enters_stake_required_after_key_bound() {
        let (_signer, state, _key_id) = validator_state();
        let mut manager = ValidatorLifecycleManager::default();
        manager.insert(state);
        manager.key_bound_requires_stake("validator-1").unwrap();
        assert_eq!(
            manager.state("validator-1").unwrap().validator.status,
            ValidatorStatus::StakeRequired
        );
    }

    #[test]
    fn validator_cannot_proceed_without_confirmed_stake() {
        let (_signer, state, _key_id) = validator_state();
        let mut manager = ValidatorLifecycleManager::default();
        manager.insert(state);
        manager.key_bound_requires_stake("validator-1").unwrap();
        assert!(manager
            .advance_after_stake(
                "validator-1",
                ValidatorStatus::Syncing,
                &ProtocolConfig::testnet_v2()
            )
            .is_err());
        assert!(!manager.can_vote("validator-1"));
        assert!(!manager.can_propose("validator-1"));
    }

    #[test]
    fn under_stake_is_rejected_and_exact_stake_submission_is_accepted() {
        let (mut signer, state, key_id) = validator_state();
        let verifier = signer.verifier();
        let mut manager = ValidatorLifecycleManager::default();
        manager.insert(state);
        manager.key_bound_requires_stake("validator-1").unwrap();
        let under = signed_stake_tx(&mut signer, &key_id, REQUIRED_VALIDATOR_STAKE_NWEI - 1);
        assert!(manager
            .submit_stake(
                "validator-1",
                stake_record(REQUIRED_VALIDATOR_STAKE_NWEI - 1),
                &under,
                &verifier
            )
            .is_err());

        let exact = signed_stake_tx(&mut signer, &key_id, REQUIRED_VALIDATOR_STAKE_NWEI);
        manager
            .submit_stake(
                "validator-1",
                stake_record(REQUIRED_VALIDATOR_STAKE_NWEI),
                &exact,
                &verifier,
            )
            .unwrap();
        assert_eq!(
            manager.state("validator-1").unwrap().validator.status,
            ValidatorStatus::StakeSubmitted
        );
    }
}
