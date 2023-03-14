use super::halo2_circuit::*;

// use group::{ff::Field, prime::PrimeCurve};
use halo2_proofs::{
    dev::MockProver,
    plonk::create_proof,
    halo2curves::FieldExt,
};

use crate::{
  circuit::{
    cs::*,
  },
  ff_uint::{PrimeField},
};

use super::{
    fawkes_cs_to_halo,
};

/// This runs a `MockProver` on a `BuildCS` value. Returns `true` if circuit
/// was built and verified correctly. The `Fy` type parameter specifies the
/// field type that the numbers in `BuildCS<Fx>` should be converted to.
pub fn mock_prove<Fx: PrimeField, Fy: FieldExt>(cs: BuildCS<Fx>) -> bool {
    use std::cmp::max;

    // Maximum number of halo2 rows. It limits the allowed number of gates and
    // inputs for our circuit. Shouldn't be greater than 2^18.
    //
    // TODO: We may need to increase this value a bit, since halo2's Layouter
    // may not fit our values perfectly, or may use a couple of rows for its
    // own stuff.
    let k = max(cs.gates.len(), cs.public.len()) as u32;

    let (cs, ins) = fawkes_cs_to_halo::<Fx, Fy>(cs);
    let ins = ins.into_iter().map(|i| i.unwrap()).collect();
    let prover = MockProver::run(k, &cs, vec![ins]).unwrap();
    prover.verify().is_ok()
}
