pub mod halo2_circuit;
pub mod setup;
pub mod prover;
pub mod verifier;
pub mod standard_plonk_config;

use crate::{ff_uint::{Num, PrimeField}, circuit::cs::BuildCS};
use self::halo2_circuit::*;
use ff_uint::NumRepr;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Chip, Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Any, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector},
};

use self::halo2_circuit::HaloCS;

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

/// Takes constraints in BuildCS format, produces a HaloCS and inputs vector
/// which can be fed to halo2 prover.
pub fn fawkes_cs_to_halo<Fx: PrimeField, Fy: FieldExt>(
    cs: BuildCS<Fx>
) -> (HaloCS<Fy>, Vec<Option<Fy>>) {
    // TODO: Some .clone() operations in this implementation are
    // unnecessary. Remove them.

    let public = {
        let mut p = cs.public;
        p.sort();
        p
    };
    let values: Vec<Option<Fy>> = cs.values
        .into_iter()
        .map(
            |v| v.map(
                |u| num_to_halo_fp(u)
            )
        ).collect();

    let g : Vec<_> = {
        let get_value = |i: usize| {
            use std::ops::Index;
            let x: &Option<Fy> = values.index(i);
            match public.binary_search(&&i) {
                Ok(i) => ValueReference::new_instance(i),
                Err(_) => ValueReference::new_advice(
                    match x {
                        None => Value::<Fy>::unknown(),
                        Some(x) => Value::known(x.clone()),
                    }
                ),
            }
        };

        cs.gates.iter().map(|g| {
            FawkesGateValues {
                x: get_value(g.x),
                y: get_value(g.y),
                z: get_value(g.z),
                a: num_to_halo_fp(g.a),
                b: num_to_halo_fp(g.b),
                c: num_to_halo_fp(g.c),
                d: num_to_halo_fp(g.d),
                e: num_to_halo_fp(g.e),
            }
        }).collect()
    };

    let ins = public.iter().map(|&i| values[i]).collect();

    (HaloCS { gates: g }, ins)
}
