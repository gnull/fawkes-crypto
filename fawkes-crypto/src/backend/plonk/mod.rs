pub mod halo2_circuit;
pub mod setup;
pub mod prover;
pub mod verifier;
pub mod standard_plonk_config;

use crate::ff_uint::{Num, PrimeField};
use ff_uint::NumRepr;
use halo2_proofs::arithmetic::FieldExt

;

pub fn num_to_halo_fp<Fx: PrimeField, Fy: FieldExt>(
    from: Num<Fx>,
) -> Fy {
    let buff = from.to_uint().into_inner();
    let buff_ref = buff.as_ref();

    let mut to = Fy::Repr::default();
    let to_ref = to.as_mut();

    assert!(buff_ref.len()*8 == to_ref.len());

    for i in 0..buff_ref.len() {
        to_ref[8*i..].copy_from_slice(&buff_ref[i].to_le_bytes());
    }

    Fy::from_repr_vartime(to).unwrap()
}

pub fn halo_fp_to_num<Fx: PrimeField, Fy: FieldExt>(
    from: Fy,
) -> Num<Fx> {
    let repr = from.to_repr();
    let buff_ref = repr.as_ref();

    let mut to = NumRepr::<Fx::Inner>::ZERO;
    let to_ref = to.as_inner_mut().as_mut();

    assert!(buff_ref.len() == to_ref.len()*8);

    for i in 0..to_ref.len() {
        to_ref[i] = u64::from_le_bytes(buff_ref[8*i..8*(i+1)].try_into().unwrap());
    }
    
    Num::from_uint(to).unwrap()
}