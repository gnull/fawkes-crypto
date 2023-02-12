use std::{marker::PhantomData, iter};

use group::{ff::Field, prime::PrimeCurve};
use halo2_proofs::{
    circuit::{AssignedCell, Chip, Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Any, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector},
    poly::Rotation, // dev::metadata::Column,
};

use crate::{
  circuit::{
    cs::*,
    num::*,
    bool::*,
  },
  ff_uint::{Num, PrimeField},
};

/// Just like `Gate`, but with concrete `F` values in place and wrapped in
/// `Value`. The `x`, `y` and `z` are allowed to be missing since they are from
/// advice, while the fixed fields must have concrete values.
#[derive(Clone, Debug)]
pub struct FawkesGateValues<F: Field + PrimeField> {
    x: Value<F>,
    y: Value<F>,
    z: Value<F>,
    a: F,
    b: F,
    c: F,
    d: F,
    e: F,
}

impl<F: Field + PrimeField> FawkesGateValues<F> {
    fn extract_gates(cs: &BuildCS<F>) -> Vec<Self> {
        use std::ops::Index;
        // Sort the vector for quick binary search
        let public: Vec<_> = itertools::sorted(cs.public.iter()).collect();
        let values = &cs.values;

        let get_value = |i| {
            let x: &Option<Num<F>> = values.index(i);
            match x {
                None => Value::unknown(),
                Some(x) => Value::known(x.0),
            }
        };

        cs.gates.iter().map(|g| {
            FawkesGateValues {
                x: get_value(g.x),
                y: get_value(g.y),
                z: get_value(g.z),
                a: g.a.0,
                b: g.b.0,
                c: g.c.0,
                d: g.d.0,
                e: g.e.0
            }
        }).collect()
    }
}

/// a*x + b*y + c*z + d*x*y + e == 0
#[derive(Clone, Debug)]
pub struct FawkesGateConfig<F: Field + PrimeField> {
    x: Column<Advice>,
    y: Column<Advice>,
    z: Column<Advice>,
    a: Column<Fixed>,
    b: Column<Fixed>,
    c: Column<Fixed>,
    d: Column<Fixed>,
    e: Column<Fixed>,
    /// Selector that enables/disables the equation for a specific row
    sel: Selector,
    _marker: PhantomData<F>,
}

impl<F: Field + PrimeField> FawkesGateConfig<F> {
    /// Allocate the columns this gate will be using, and describe the
    /// constraint equation it will enforce. (Without knowing the cell values
    /// or the rows that we will occupy yet.)
    fn config(meta: &mut ConstraintSystem<F>) -> Self {
        // We allocate the columns over which we will be defining our gate. We
        // also enable equality constraints for each of the three advice gates.
        let res = {
            let make_advice = &mut || {
                let c = meta.advice_column();
                meta.enable_equality(c);
                c
            };

            Self {
                x: make_advice(),
                y: make_advice(),
                z: make_advice(),
                a: meta.fixed_column(),
                b: meta.fixed_column(),
                c: meta.fixed_column(),
                d: meta.fixed_column(),
                e: meta.fixed_column(),
                sel: meta.selector(),
                _marker: PhantomData,
            }
        };

        // This call describes the shape of our gate over matrix cells. Here,
        // we know neither the concrete advice/instance/selector values, nor
        // the row in which the gate will be placed yet (such things are
        // determined at synthesis time).
        meta.create_gate("standard_gate", |virtual_cells| {
            // Query the cells that are at the intersection of the current
            // (virtual) row and each of the columns that we just allocated.
            let sel = virtual_cells.query_selector(res.sel);
            let x = virtual_cells.query_advice(res.x, Rotation::cur());
            let y = virtual_cells.query_advice(res.y, Rotation::cur());
            let z = virtual_cells.query_advice(res.z, Rotation::cur());
            let a = virtual_cells.query_fixed(res.a, Rotation::cur());
            let b = virtual_cells.query_fixed(res.b, Rotation::cur());
            let c = virtual_cells.query_fixed(res.c, Rotation::cur());
            let d = virtual_cells.query_fixed(res.d, Rotation::cur());
            let e = virtual_cells.query_fixed(res.e, Rotation::cur());

            // Produce the constraint for the current row. We require that the
            // expression given in the brackets equals 0.
            vec![sel * (a * x.clone() + b * y.clone() + c * z + d * x * y + e)]
        });

        res
    }

    // TODO: ensure this function adds all the necessary copy constraints (when
    // the same advice value is reused) to make sure that adversarial prover
    // can't cheat.
    //
    // Also: correctly expose inputs.
    fn synthesize(
        &self,
        mut layouter: impl Layouter<F>,
        g: &FawkesGateValues<F>
    ) -> Result<(), Error> {
        layouter.assign_region(|| format!("synthesize {:?}", g), |mut region| {
            // TODO: Figure out what is this offset value exactly
            let offset = 0;

            // Enable constraint
            self.sel.enable(&mut region, offset)?;

            // Assign the advice values in the current row. Save the
            region.assign_advice(|| format!("x = {:?}", g.x), self.x, offset, || g.x)?;
            region.assign_advice(|| format!("y = {:?}", g.y), self.y, offset, || g.y)?;
            region.assign_advice(|| format!("z = {:?}", g.z), self.z, offset, || g.z)?;

            // Assign the fixed values in the current row
            region.assign_fixed(|| format!("a = {:?}", g.a), self.a, offset, || Value::known(g.a))?;
            region.assign_fixed(|| format!("b = {:?}", g.b), self.b, offset, || Value::known(g.b))?;
            region.assign_fixed(|| format!("c = {:?}", g.c), self.c, offset, || Value::known(g.c))?;
            region.assign_fixed(|| format!("d = {:?}", g.d), self.d, offset, || Value::known(g.d))?;
            region.assign_fixed(|| format!("e = {:?}", g.e), self.e, offset, || Value::known(g.e))?;

            Ok(())
        })
    }
}

impl<F: Field + PrimeField> Circuit<F> for BuildCS<F> {
    type Config = FawkesGateConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        BuildCS {
            values: self.values.iter().map(|_| None).collect(),
            gates: self.gates.clone(),
            tracking: self.tracking,
            public: self.public.clone(),
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FawkesGateConfig::config(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>
    ) -> Result<(), Error> {
        let gates = FawkesGateValues::extract_gates(self);
        for (i, g) in gates.iter().enumerate() {
            config.synthesize(layouter.namespace(|| format!("gate #{}", i)), g)?
        }
        Ok(())
    }
}
