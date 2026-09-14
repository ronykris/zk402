use ark_bls12_381::{Bls12_381, Fr as ConstraintF};
use ark_crypto_primitives::snark::{CircuitSpecificSetupSNARK, SNARK};
use ark_ec::{CurveGroup, Group};
use ark_ed_on_bls12_381::{EdwardsAffine, EdwardsProjective, Fr as JubjubScalar};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{prepare_verifying_key, Groth16, Proof, ProvingKey, VerifyingKey};
use ark_r1cs_std::{
    alloc::AllocVar,
    boolean::Boolean,
    eq::EqGadget,
    groups::{curves::twisted_edwards::AffineVar, CurveVar},
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use ark_std::rand::{CryptoRng, RngCore};
use core::fmt;
use sha2::{Digest, Sha512};

type EdwardsVar =
    AffineVar<ark_ed_on_bls12_381::JubjubConfig, ark_ed_on_bls12_381::constraints::FqVar>;
type ZkGroth16 = Groth16<Bls12_381>;
const SCALAR_BITS: usize = 256;

#[derive(Debug)]
pub enum Error {
    Encoding(SerializationError),
    Synthesis(SynthesisError),
    Snark(ark_crypto_primitives::Error),
}

pub type Result<T> = core::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding(err) => write!(f, "invalid byte encoding: {err}"),
            Self::Synthesis(err) => write!(f, "proving failed: {err}"),
            Self::Snark(err) => write!(f, "snark error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<SerializationError> for Error {
    fn from(err: SerializationError) -> Self {
        Self::Encoding(err)
    }
}

impl From<SynthesisError> for Error {
    fn from(err: SynthesisError) -> Self {
        Self::Synthesis(err)
    }
}

impl From<ark_crypto_primitives::Error> for Error {
    fn from(err: ark_crypto_primitives::Error) -> Self {
        Self::Snark(err)
    }
}

#[derive(Clone)]
struct SignatureKnowledgeCircuit {
    public_key: Option<EdwardsAffine>,
    challenge_bits: Vec<bool>,
    secret_key_bits: Option<Vec<bool>>,
    nonce_bits: Option<Vec<bool>>,
    response_bits: Option<Vec<bool>>,
}

impl ConstraintSynthesizer<ConstraintF> for SignatureKnowledgeCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<ConstraintF>,
    ) -> core::result::Result<(), SynthesisError> {
        let public_key = EdwardsVar::new_input(cs.clone(), || {
            self.public_key.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let challenge_bits = allocate_input_bits(cs.clone(), &self.challenge_bits)?;
        let secret_key_bits = allocate_witness_bits(cs.clone(), self.secret_key_bits)?;
        let nonce_bits = allocate_witness_bits(cs.clone(), self.nonce_bits)?;
        let response_bits = allocate_witness_bits(cs, self.response_bits)?;

        let base = EdwardsVar::constant(EdwardsProjective::generator());
        let derived_public_key = base.scalar_mul_le(secret_key_bits.iter())?;
        derived_public_key.enforce_equal(&public_key)?;

        let nonce_point = base.scalar_mul_le(nonce_bits.iter())?;
        let response_point = base.scalar_mul_le(response_bits.iter())?;
        let challenged_public_key = public_key.scalar_mul_le(challenge_bits.iter())?;
        let expected_response_point = nonce_point + challenged_public_key;

        response_point.enforce_equal(&expected_response_point)?;
        Ok(())
    }
}

/// Generate a circuit-specific Groth16 proving key and verifying key.
pub fn setup<R: RngCore + CryptoRng>(rng: &mut R) -> Result<(Vec<u8>, Vec<u8>)> {
    let one = JubjubScalar::from_le_bytes_mod_order(&[1]);
    let blank = SignatureKnowledgeCircuit {
        public_key: Some(EdwardsProjective::generator().into_affine()),
        challenge_bits: vec![false; SCALAR_BITS],
        secret_key_bits: Some(scalar_bits_le(&one)),
        nonce_bits: Some(scalar_bits_le(&one)),
        response_bits: Some(scalar_bits_le(&one)),
    };
    let (pk, vk) = ZkGroth16::setup(blank, rng)?;
    Ok((serialize_compressed(&pk)?, serialize_compressed(&vk)?))
}

/// Derive the compressed Jubjub public key for a raw private key byte string.
pub fn derive_public_key(secret_key: &[u8]) -> Result<Vec<u8>> {
    let secret = scalar_from_bytes(secret_key);
    serialize_compressed(&(EdwardsProjective::generator() * secret).into_affine())
}

/// Create a Groth16 proof for a hidden Schnorr/EdDSA-style signature over `message`.
///
/// The proof public inputs are the derived public key and a challenge derived
/// from `message`; the nonce point and response scalar are private witnesses.
pub fn prove<R: RngCore + CryptoRng>(
    proving_key: &[u8],
    secret_key: &[u8],
    message: &[u8],
    nonce: &[u8],
    rng: &mut R,
) -> Result<Vec<u8>> {
    let pk = ProvingKey::<Bls12_381>::deserialize_compressed(proving_key)?;
    let secret = scalar_from_bytes(secret_key);
    let nonce = scalar_from_bytes(nonce);
    let public_key = (EdwardsProjective::generator() * secret).into_affine();
    let challenge = challenge_scalar(&public_key, message)?;
    let response = nonce + challenge * secret;

    let circuit = SignatureKnowledgeCircuit {
        public_key: Some(public_key),
        challenge_bits: scalar_bits_le(&challenge),
        secret_key_bits: Some(scalar_bits_le(&secret)),
        nonce_bits: Some(scalar_bits_le(&nonce)),
        response_bits: Some(scalar_bits_le(&response)),
    };

    let proof = ZkGroth16::prove(&pk, circuit, rng)?;
    serialize_compressed(&proof)
}

/// Verify a Groth16 proof against raw public-key and message bytes.
pub fn verify(
    verifying_key: &[u8],
    proof: &[u8],
    public_key: &[u8],
    message: &[u8],
) -> Result<bool> {
    let vk = VerifyingKey::<Bls12_381>::deserialize_compressed(verifying_key)?;
    let proof = Proof::<Bls12_381>::deserialize_compressed(proof)?;
    let public_key = EdwardsAffine::deserialize_compressed(public_key)?;
    let challenge = challenge_scalar(&public_key, message)?;
    let public_inputs = groth16_public_inputs(&public_key, &challenge);
    let pvk = prepare_verifying_key(&vk);
    Ok(ZkGroth16::verify_with_processed_vk(
        &pvk,
        &public_inputs,
        &proof,
    )?)
}

fn allocate_input_bits(
    cs: ConstraintSystemRef<ConstraintF>,
    bits: &[bool],
) -> core::result::Result<Vec<Boolean<ConstraintF>>, SynthesisError> {
    bits.iter()
        .map(|bit| Boolean::new_input(cs.clone(), || Ok(*bit)))
        .collect()
}

fn allocate_witness_bits(
    cs: ConstraintSystemRef<ConstraintF>,
    bits: Option<Vec<bool>>,
) -> core::result::Result<Vec<Boolean<ConstraintF>>, SynthesisError> {
    let bits = bits.ok_or(SynthesisError::AssignmentMissing)?;
    bits.into_iter()
        .map(|bit| Boolean::new_witness(cs.clone(), || Ok(bit)))
        .collect()
}

fn scalar_from_bytes(bytes: &[u8]) -> JubjubScalar {
    JubjubScalar::from_le_bytes_mod_order(bytes)
}

fn challenge_scalar(public_key: &EdwardsAffine, message: &[u8]) -> Result<JubjubScalar> {
    let public_key_bytes = serialize_compressed(public_key)?;
    let mut hasher = Sha512::new();
    hasher.update(b"zk402.phase1.challenge.v1");
    hasher.update(public_key_bytes);
    hasher.update(message);
    Ok(JubjubScalar::from_le_bytes_mod_order(&hasher.finalize()))
}

fn scalar_bits_le(scalar: &JubjubScalar) -> Vec<bool> {
    let mut bits = scalar.into_bigint().to_bits_le();
    bits.resize(SCALAR_BITS, false);
    bits
}

fn groth16_public_inputs(public_key: &EdwardsAffine, challenge: &JubjubScalar) -> Vec<ConstraintF> {
    let mut inputs = vec![public_key.x, public_key.y];
    inputs.extend(
        scalar_bits_le(challenge)
            .into_iter()
            .map(|bit| ConstraintF::from_le_bytes_mod_order(&[u8::from(bit)])),
    );
    inputs
}

fn serialize_compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(value.compressed_size());
    value.serialize_compressed(&mut bytes)?;
    Ok(bytes)
}

// ProofSystem trait implementation for zk402-core integration
pub mod proof_system {
    use super::*;
    use zk402_core::{ProofSystem, ProofSystemError, PublicInputs, Witness};

    pub struct Groth16ProofSystem;

    impl ProofSystem for Groth16ProofSystem {
        type ProvingKey = Vec<u8>;
        type VerifyingKey = Vec<u8>;

        fn prove(
            pk: &Self::ProvingKey,
            witness: &Witness,
            public_inputs: &PublicInputs,
        ) -> core::result::Result<Vec<u8>, ProofSystemError> {
            let message = construct_message(public_inputs);
            let mut rng = ark_std::rand::thread_rng();

            super::prove(
                pk,
                &witness.secret_key,
                &message,
                &witness.nonce_scalar,
                &mut rng,
            )
            .map_err(|e| ProofSystemError::ProvingFailed(e.to_string()))
        }

        fn verify(
            vk: &Self::VerifyingKey,
            proof: &[u8],
            public_inputs: &PublicInputs,
        ) -> core::result::Result<bool, ProofSystemError> {
            let message = construct_message(public_inputs);

            super::verify(vk, proof, &public_inputs.public_key, &message)
                .map_err(|e| ProofSystemError::VerificationFailed(e.to_string()))
        }
    }

    fn construct_message(inputs: &PublicInputs) -> Vec<u8> {
        // Construct message from payment parameters following Phase 1 approach
        let mut message = Vec::new();
        message.extend_from_slice(b"zk402.payment.v1:");
        message.extend_from_slice(inputs.from.as_slice());
        message.extend_from_slice(inputs.pay_to.as_slice());
        message.extend_from_slice(inputs.amount.as_bytes());
        message.extend_from_slice(inputs.asset.as_slice());
        message.extend_from_slice(inputs.network.as_bytes());
        message.extend_from_slice(&inputs.nonce);
        message.extend_from_slice(&inputs.valid_after.to_le_bytes());
        message.extend_from_slice(&inputs.valid_before.to_le_bytes());
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::rand::{rngs::StdRng, SeedableRng};
    use std::sync::OnceLock;

    fn message(amount: &str, pay_to: &str) -> Vec<u8> {
        format!("amount={amount};pay_to={pay_to}").into_bytes()
    }

    fn test_keys() -> &'static (Vec<u8>, Vec<u8>) {
        static KEYS: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
        KEYS.get_or_init(|| {
            let mut rng = StdRng::seed_from_u64(1);
            setup(&mut rng).unwrap()
        })
    }

    fn proof_for(message: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut rng = StdRng::seed_from_u64(1);
        let secret_key = b"phase-one-test-secret-key";
        let nonce = b"phase-one-test-nonce";
        let public_key = derive_public_key(secret_key).unwrap();
        let proof = prove(&test_keys().0, secret_key, message, nonce, &mut rng).unwrap();
        (test_keys().1.clone(), public_key, proof)
    }

    #[test]
    fn valid_signature_knowledge_proof_verifies() {
        let msg = message("100", "0xabc");
        let (vk, public_key, proof) = proof_for(&msg);

        assert!(verify(&vk, &proof, &public_key, &msg).unwrap());
    }

    #[test]
    fn tampered_amount_fails_to_verify() {
        let (vk, public_key, proof) = proof_for(&message("100", "0xabc"));

        assert!(!verify(&vk, &proof, &public_key, &message("101", "0xabc")).unwrap());
    }

    #[test]
    fn tampered_pay_to_fails_to_verify() {
        let (vk, public_key, proof) = proof_for(&message("100", "0xabc"));

        assert!(!verify(&vk, &proof, &public_key, &message("100", "0xdef")).unwrap());
    }
}
