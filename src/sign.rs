use rand::Rng;
use rug::Integer;
use sha256::digest;

use crate::{modular, FrostState};

pub struct PublicCommitment {
    pub participant_id: Integer,
    pub di: Integer,
    pub ei: Integer,
    pub public_share: Integer,
}

impl PublicCommitment {
    pub fn new(participant_id: Integer, di: Integer, ei: Integer, public_share: Integer) -> Self {
        Self {
            participant_id,
            di,
            ei,
            public_share,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}::{}::{}", self.participant_id, self.di, self.ei)
    }
}

pub fn compute_binding_value(
    state: &FrostState,
    participant_commitment: &PublicCommitment,
    message: &str,
) -> Integer {
    Integer::from_str_radix(
        digest(format!(
            "{}::::{}::::{}",
            participant_commitment.participant_id,
            message,
            participant_commitment.to_string()
        ))
        .as_str(),
        16,
    )
    .unwrap()
    .modulo(&state.q)
}

pub fn compute_group_commitment_and_challenge(
    state: &FrostState,
    participants_commitments: &[PublicCommitment],
    message: &str,
    group_public_key: Integer,
) -> (Integer, Integer) {
    let group_commitment = participants_commitments
        .iter()
        .fold(Integer::from(1), |acc, pc| {
            let binding_value = compute_binding_value(&state, &pc, &message);
            modular::mul(
                modular::mul(acc.clone(), pc.di.clone(), &state.q),
                modular::pow(&pc.ei, &binding_value, &state.q),
                &state.q,
            )
        });
    let challenge = Integer::from_str_radix(
        digest(format!(
            "{}::::{}::::{}",
            group_commitment, group_public_key, message
        ))
        .as_str(),
        16,
    )
    .unwrap()
    .modulo(&state.q);
    (group_commitment, challenge)
}

pub fn lagrange_coefficient(
    state: &FrostState,
    participant_id: &Integer,
    number_of_participants: u32,
) -> Integer {
    (0..(number_of_participants)).fold(Integer::from(1), |acc, j| {
        let j = Integer::from(j);
        modular::mul(
            acc.clone(),
            modular::div(
                j.clone(),
                modular::sub(j, participant_id.clone(), &state.q),
                &state.q,
            ),
            &state.q,
        )
    })
}

pub fn compute_own_response(
    state: &FrostState,
    participant_commitment: &PublicCommitment,
    private_key: &Integer,
    nonces: &(Integer, Integer),
    lagrange_coefficient: &Integer,
    challenge: &Integer,
    message: &str,
) -> Integer {
    let binding_value = compute_binding_value(&state, &participant_commitment, &message);
    let (di, ei) = nonces;
    modular::add(
        di.clone(),
        modular::add(
            modular::mul(ei.clone(), binding_value, &state.q),
            modular::mul(
                modular::mul(lagrange_coefficient.clone(), private_key.clone(), &state.q),
                challenge.clone(),
                &state.q,
            ),
            &state.q,
        ),
        &state.q,
    )
}

pub fn verify_participants(
    state: &FrostState,
    participants_commitments: &[PublicCommitment],
    message: &str,
    own_response: &Integer,
    challenge: &Integer,
    number_of_participants: u32,
) -> bool {
    let gz = modular::pow(&state.generator, &own_response, &state.q);
    participants_commitments.iter().fold(true, |acc, pc| {
        let binding_value = compute_binding_value(&state, &pc, &message);
        let ri = modular::mul(
            pc.di.clone(),
            modular::pow(&pc.ei, &binding_value, &state.q),
            &state.q,
        );
        let to_validate = modular::mul(
            ri,
            modular::pow(
                &pc.public_share,
                &modular::mul(
                    challenge.clone(),
                    lagrange_coefficient(&state, &pc.participant_id, number_of_participants),
                    &state.q,
                ),
                &state.q,
            ),
            &state.q,
        );
        assert_eq!(to_validate, gz, "Failed to validate the participants.");
        acc && (to_validate == gz)
    })
}

pub fn compute_aggregate_response(
    state: &FrostState,
    participants_responses: &[Integer],
) -> Integer {
    participants_responses
        .iter()
        .fold(Integer::from(0), |acc, pr| {
            modular::add(acc, pr.clone(), &state.q)
        })
}
