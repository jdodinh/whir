use ark_crypto_primitives::merkle_tree::{Config, MultiPath};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

pub mod committer;
pub mod domainsep;
pub mod parameters;
pub mod prover;
pub mod verifier;

// Only includes the authentication paths
#[derive(Clone, CanonicalSerialize, CanonicalDeserialize)]
pub struct StirProof<F, MerkleConfig>
where
    F: Field + Sized + Clone + CanonicalSerialize + CanonicalDeserialize,
    MerkleConfig: Config<Leaf = [F]>,
{
    merkle_proofs: Vec<(MultiPath<MerkleConfig>, Vec<Vec<F>>)>,
}

pub fn stir_proof_size<MerkleConfig, F>(
    transcript: &[u8],
    stir_proof: &StirProof<F, MerkleConfig>,
) -> usize
where
    F: Field + Sized + Clone + CanonicalSerialize + CanonicalDeserialize,
    MerkleConfig: Config<Leaf = [F]>,
{
    transcript.len() + stir_proof.serialized_size(ark_serialize::Compress::Yes)
}

#[cfg(test)]
mod tests {
    use ark_poly::univariate::DensePolynomial;
    use ark_poly::DenseUVPolynomial;
    use spongefish::{DefaultHash, DomainSeparator};
    use spongefish_pow::blake3::Blake3PoW;

    use crate::{
        crypto::{
            fields::Field64,
            merkle_tree::{
                blake3::{Blake3Compress, Blake3LeafHash, Blake3MerkleTreeParams},
                parameters::default_config,
            },
        },
        parameters::{
            FoldType, FoldingFactor, ProtocolParameters, SoundnessType, UnivariateParameters,
        },
        stir_ldt::{
            committer::CommitmentWriter, domainsep::StirDomainSeparator, parameters::StirConfig,
            prover::Prover, verifier::Verifier,
        },
    };

    type MerkleConfig = Blake3MerkleTreeParams<F>;
    type PowStrategy = Blake3PoW;
    type F = Field64;

    fn make_stir_things(
        folding_factor: usize,
        log_degree: usize,
        fold_type: FoldType,
        soundness_type: SoundnessType,
        pow_bits: usize,
    ) {
        dbg!((
            folding_factor,
            log_degree,
            fold_type,
            soundness_type,
            pow_bits,
        ));

        let num_coeffs = 1 << log_degree;

        let mut rng = ark_std::test_rng();
        let (leaf_hash_params, two_to_one_params) =
            default_config::<F, Blake3LeafHash<F>, Blake3Compress>(&mut rng);

        let mv_params = UnivariateParameters::<F>::new(log_degree);

        let stir_params = ProtocolParameters::<MerkleConfig, PowStrategy> {
            initial_statement: false,
            security_level: 32,
            pow_bits,
            folding_factor: FoldingFactor::Constant(folding_factor),
            leaf_hash_params,
            two_to_one_params,
            fold_optimisation: fold_type,
            soundness_type,
            starting_log_inv_rate: 1,
            _pow_parameters: Default::default(),
        };

        let params = StirConfig::<F, MerkleConfig, PowStrategy>::new(mv_params, stir_params);

        let polynomial = DensePolynomial::from_coefficients_vec(vec![F::from(1); num_coeffs]);

        let domainsep = DomainSeparator::<DefaultHash>::new("🌪️")
            .commit_statement(&params)
            .add_stir_proof(&params)
            .clone();

        let mut prover_state = domainsep.to_prover_state();

        let committer = CommitmentWriter::new(params.clone());
        let witness = committer.commit(&mut prover_state, polynomial).unwrap();

        let prover = Prover::new(params.clone());

        let proof = prover.prove(&mut prover_state, &witness).unwrap();

        let verifier = Verifier::new(&params);
        let mut arthur = domainsep.to_verifier_state(prover_state.narg_string());
        assert!(verifier.verify(&mut arthur, &proof,).is_ok());
    }

    #[test]
    fn test_stir_ldt_large_instance() {
        let folding_factors = [4, 5];
        let log_degree = 16;
        let soundness_types = [
            SoundnessType::ConjectureList,
            SoundnessType::ProvableList,
            SoundnessType::UniqueDecoding,
        ];
        let fold_types = [FoldType::Naive, FoldType::ProverHelps];
        let pow_bitss = [0, 5, 10];
        for folding_factor in folding_factors {
            for soundness_type in soundness_types {
                for fold_type in fold_types {
                    for pow_bits in pow_bitss {
                        make_stir_things(
                            folding_factor,
                            log_degree,
                            fold_type,
                            soundness_type,
                            pow_bits,
                        );
                    }
                }
            }
        }
    }

    // This test is ignored because currently the parameters do not satisfy required bounds.
    // We keep the code of this test because it is a good idea to test it this way in the future.
    // #[ignore]
    #[test]
    fn test_stir_ldt() {
        let folding_factors = [1, 2, 3, 4];
        let fold_types = [FoldType::Naive, FoldType::ProverHelps];
        let soundness_type = [
            SoundnessType::ConjectureList,
            SoundnessType::ProvableList,
            SoundnessType::UniqueDecoding,
        ];
        let pow_bits = [0, 5, 10];

        for folding_factor in folding_factors {
            let num_variables = folding_factor..=3 * folding_factor;
            for num_variables in num_variables {
                for fold_type in fold_types {
                    for soundness_type in soundness_type {
                        for pow_bits in pow_bits {
                            make_stir_things(
                                folding_factor,
                                num_variables,
                                fold_type,
                                soundness_type,
                                pow_bits,
                            );
                        }
                    }
                }
            }
        }
    }
}
