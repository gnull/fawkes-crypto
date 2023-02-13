use super::halo2_circuit::*;

use group::{ff::Field, prime::PrimeCurve};
use halo2_proofs::{
    circuit::{AssignedCell, Chip, Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Any, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector},
    poly::Rotation, dev::MockProver, // dev::metadata::Column,
};

use crate::{
  circuit::{
    cs::*,
    num::*,
    bool::*,
  },
  ff_uint::{Num, PrimeField},
};

/// This runs a `MockProver` on a `BuildCS` value. Returns `Ok(true)` if
/// circuit was built and verified correctly.
pub fn mock_prove<F: Field + PrimeField + Ord>(k: u32, cs: &BuildCS<F>) -> bool {
    let inputs = extract_inputs(cs).into_iter().map(|i| i.unwrap().0).collect();
    let prover = MockProver::run(k, cs, vec![inputs]).unwrap();
    prover.verify().is_ok()
}
