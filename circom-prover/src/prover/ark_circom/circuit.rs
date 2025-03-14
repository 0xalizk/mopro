// This file is copied from https://github.dev/zkmopro/circom-compat/tree/wasm-delete

use anyhow::Result;
use ark_ec::pairing::Pairing;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};

use super::r1cs_reader::R1CS;

#[derive(Clone, Debug)]
pub struct CircomCircuit<E: Pairing> {
    pub r1cs: R1CS<E>,
    pub witness: Option<Vec<E::ScalarField>>,
}

impl<E: Pairing> CircomCircuit<E> {
    pub fn get_public_inputs(&self) -> Option<Vec<E::ScalarField>> {
        match &self.witness {
            None => None,
            Some(w) => match &self.r1cs.wire_mapping {
                None => Some(w[1..self.r1cs.num_inputs].to_vec()),
                Some(m) => Some(m[1..self.r1cs.num_inputs].iter().map(|i| w[*i]).collect()),
            },
        }
    }
}

impl<E: Pairing> ConstraintSynthesizer<E::ScalarField> for CircomCircuit<E> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<E::ScalarField>,
    ) -> Result<(), SynthesisError> {
        let witness = &self.witness;
        let wire_mapping = &self.r1cs.wire_mapping;

        // Start from 1 because Arkworks implicitly allocates One for the first input
        for i in 1..self.r1cs.num_inputs {
            cs.new_input_variable(|| {
                Ok(match witness {
                    None => E::ScalarField::from(1u32),
                    Some(w) => match wire_mapping {
                        Some(m) => w[m[i]],
                        None => w[i],
                    },
                })
            })?;
        }

        for i in 0..self.r1cs.num_aux {
            cs.new_witness_variable(|| {
                Ok(match witness {
                    None => E::ScalarField::from(1u32),
                    Some(w) => match wire_mapping {
                        Some(m) => w[m[i + self.r1cs.num_inputs]],
                        None => w[i + self.r1cs.num_inputs],
                    },
                })
            })?;
        }

        let make_index = |index| {
            if index < self.r1cs.num_inputs {
                Variable::Instance(index)
            } else {
                Variable::Witness(index - self.r1cs.num_inputs)
            }
        };
        let make_lc = |lc_data: &[(usize, E::ScalarField)]| {
            lc_data.iter().fold(
                LinearCombination::<E::ScalarField>::zero(),
                |lc: LinearCombination<E::ScalarField>, (index, coeff)| {
                    lc + (*coeff, make_index(*index))
                },
            )
        };

        for constraint in &self.r1cs.constraints {
            cs.enforce_constraint(
                make_lc(&constraint.0),
                make_lc(&constraint.1),
                make_lc(&constraint.2),
            )?;
        }

        Ok(())
    }
}
