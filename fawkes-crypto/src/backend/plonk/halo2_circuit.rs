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

use either::Either::*;

/// We use this struct to hold a value of Instance or Advice element while
/// synthesizing the halo2 circuit. Initially, `ValueReference` holds its
/// `Value` or its index in the instance vector, but after being assigned to
/// some cell, it memorizes the cell instead and all future assignments are
/// implemented via copy constraint with the original cell.
///
/// (Just directly assigning the `Value` each time is incorrrect, since that
/// can allow a malicious prover to assign different values each time.)
#[derive(Clone, Debug)]
pub enum ValueReference<F: Field + PrimeField> {
    /// The `Value` or the cell where it was assigned the first time.
    ValueAdvice(Value<F>),
    /// The index of instance element that should be assigned here.
    ValueInstance(usize),
    /// The index of a cell where this value was already assigned (so just
    /// refer to that cell via a copy constraint if you need a value)
    ValueCell(AssignedCell<F, F>),
}

impl<F: Field + PrimeField> ValueReference<F> {
    fn new_advice(v: Value<F>) -> Self {
        ValueReference::ValueAdvice(v)
    }
    fn new_instance(i: usize) -> Self {
        ValueReference::ValueInstance(i)
    }
    /// Assign this value to a region cell (given by column and offset)
    fn assign(
        &mut self,
        region: &mut Region<F>,
        instance: Column<Instance>,
        advice: Column<Advice>,
        offset: usize,
    ) -> Result<(), Error> {
        match self {
            ValueReference::ValueAdvice(v) => {
                // Assign the value for the first time
                let c = region.assign_advice(
                    || format!("{:?}", v),
                    advice,
                    offset,
                    || v.clone()
                )?;
                *self = ValueReference::ValueCell(c);
            },
            ValueReference::ValueInstance(i) => {
                // If our value is an input instance element, copy it from there
                region.assign_advice_from_instance(
                    || format!("input #{:?}", i),
                    instance,
                    *i,
                    advice,
                    offset
                )?;
            },
            ValueReference::ValueCell(c) => {
                // If we've already assigned this value somewhere, copy from there
                c.copy_advice(|| "advice copy", region, advice, offset)?;
            },
        }
        Ok(())
    }
}

/// Just like `Gate`, but with concrete `F` values in place and wrapped in
/// `Value`. The `x`, `y` and `z` are allowed to be missing since they are from
/// advice, while the fixed fields must have concrete values.
#[derive(Clone, Debug)]
pub struct FawkesGateValues<F: Field + PrimeField> {
    x: ValueReference<F>,
    y: ValueReference<F>,
    z: ValueReference<F>,
    a: F,
    b: F,
    c: F,
    d: F,
    e: F,
}

impl<F: Field + PrimeField> FawkesGateValues<F> {
    fn extract_gates(
        values: &Vec<Option<F>>,
        gates: &Vec<Gate<F>>,
        public: &Vec<usize>
    ) -> Vec<Self> {
        use std::ops::Index;
        let get_value = |i: usize| {
            let x: &Option<F> = values.index(i);
            let v = match x {
                None => Value::unknown(),
                Some(x) => Value::known(x.clone()),
            };
            match public.binary_search(&&i) {
                Ok(i) => ValueReference::new_instance(i),
                Err(_) => ValueReference::new_advice(v),
            }
        };

        gates.iter().map(|g| {
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
    /// The row where we expose inputs when we need to
    inst: Column<Instance>,
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
            let inst = meta.instance_column();
            meta.enable_equality(inst);

            let make_advice = &mut || {
                let c = meta.advice_column();
                meta.enable_equality(c);
                c
            };
            let x = make_advice();
            let y = make_advice();
            let z = make_advice();

            Self {
                x,
                y,
                z,
                a: meta.fixed_column(),
                b: meta.fixed_column(),
                c: meta.fixed_column(),
                d: meta.fixed_column(),
                e: meta.fixed_column(),
                sel: meta.selector(),
                inst,
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

    fn synthesize(
        &self,
        mut layouter: impl Layouter<F>,
        mut g: FawkesGateValues<F>
    ) -> Result<(), Error> {
        layouter.assign_region(|| format!("synthesize gate {:?}", ()), |mut region| {
            // Row offset with respect to current region. We put all the values
            // in one row, so offset is always 0.
            let offset = 0;

            // Enable constraint
            self.sel.enable(&mut region, offset)?;

            // Assign the advice values in the current row. Save the
            g.x.assign(&mut region, self.inst, self.x, offset)?;
            g.y.assign(&mut region, self.inst, self.y, offset)?;
            g.z.assign(&mut region, self.inst, self.z, offset)?;

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
        // Sort the vector for quick binary search
        let public: Vec<usize> = itertools::sorted(self.public.iter().cloned()).collect();
        // Remove Num wrappers
        let values = self.values.iter().map(|v| v.map(|Num(u)| u)).collect();

        let gates = FawkesGateValues::extract_gates(&values, &self.gates, &public);
        for (i, g) in gates.into_iter().enumerate() {
            config.synthesize(layouter.namespace(|| format!("gate #{}", i)), g)?
        }
        Ok(())
    }
}

pub fn extract_inputs<F: Field + PrimeField>(cs: &BuildCS<F>) -> Vec<Option<Num<F>>> {
    itertools::sorted(cs.public.iter().cloned())
        .map(|i| cs.values[i])
        .collect()
}
