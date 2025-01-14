use crate::{
    circuit::num::CNum,
    core::signal::Signal,
    ff_uint::{Num, PrimeField},
};

use std::{cell::RefCell, rc::Rc};

pub type RCS<Fr> = Rc<RefCell<CS<Fr>>>;

/// A `Gate` describes constraint of the form
///
/// ```
/// a*x + b*y + c*z + d*x*y + e == 0
/// ```
///
/// where `x`, `y`, `z` are variable witness elements (represented here as
/// indices), while the `a` ... `e` values are concrete constants represented
/// here as field values.
#[derive(Clone, Debug)]
pub struct Gate<Fr: PrimeField> {
    a: Num<Fr>,
    x: usize,
    b: Num<Fr>,
    y: usize,
    c: Num<Fr>,
    z: usize,
    d: Num<Fr>,
    e: Num<Fr>,
}

#[derive(Clone, Debug)]
pub struct CS<Fr: PrimeField> {
    pub values: Vec<Option<Num<Fr>>>,
    pub gates: Vec<Gate<Fr>>,
    pub tracking: bool,
    /// Indices of public witness components, i.e. the inputs.
    pub public: Vec<usize>,
}

impl<Fr: PrimeField> CS<Fr> {
    pub fn num_gate(&self) -> usize {
        self.gates.len()
    }

    pub fn new(tracking: bool) -> Self {
        Self {
            values: vec![],
            gates: vec![],
            tracking,
            public: vec![],
        }
    }

    pub fn rc_new(tracking: bool) -> RCS<Fr> {
        Rc::new(RefCell::new(Self::new(tracking)))
    }

    /// Enforce a*x + b*y + c*z + d*x*y + e == 0. This is the raw form of the
    /// constraints implemented by PLONK, the other ones like `enforce_add`
    /// and `enforce_mul` are implemented through this.
    ///
    /// One may find this not the most intuitive way to express constraints,
    /// but it's very efficient, and allows you to express things like `a + b +
    /// ab` through a single constraint.
    pub fn enforce_generic(
        x: &CNum<Fr>,
        y: &CNum<Fr>,
        z: &CNum<Fr>,
        a: &Num<Fr>,
        b: &Num<Fr>,
        c: &Num<Fr>,
        d: &Num<Fr>,
        e: &Num<Fr>
    ) {
        let mut rcs = x.get_cs().borrow_mut();
        if rcs.tracking {
            match (x.value, y.value, z.value) {
                (Some(x), Some(y), Some(z)) => {
                    assert!(
                       a*x + b*y + c*z + d*x*y + e == Num::ZERO,
                       "Not satisfied constraint"
                    );
                }
                _ => {}
            }
        }
        rcs.gates.push(Gate {
            a: a * x.0 + d * x.0 * y.2 + y.2,
            x: x.1,
            b: b * y.0 + d * y.0 * x.2 + x.2,
            y: y.1,
            c: c * z.0,
            z: z.1,
            d: d * x.0 * y.0,
            e: e + a * x.2 + b * y.2 + c * z.2 + d * x.2 * y.2,
        })
    }

    // a*b === c
    pub fn enforce_mul(x: &CNum<Fr>, y: &CNum<Fr>, z: &CNum<Fr>) {
        let mut rcs = x.get_cs().borrow_mut();
        if rcs.tracking {
            match (x.value, y.value, z.value) {
                (Some(x), Some(y), Some(z)) => {
                    assert!(x * y == z, "Not satisfied constraint");
                }
                _ => {}
            }
        }
        rcs.gates.push(Gate {
            a: x.lc.0 * y.lc.2,
            x: x.lc.1,
            b: x.lc.2 * y.lc.0,
            y: y.lc.1,
            c: -z.lc.0,
            z: z.lc.1,
            d: x.lc.0 * y.lc.0,
            e: x.lc.2 * y.lc.2 - z.lc.2,
        })
    }

    pub fn enforce_add(x: &CNum<Fr>, y: &CNum<Fr>, z: &CNum<Fr>) {
        let mut rcs = x.get_cs().borrow_mut();
        if rcs.tracking {
            match (x.value, y.value, z.value) {
                (Some(x), Some(y), Some(z)) => {
                    assert!(x + y == z, "Not satisfied constraint");
                }
                _ => {}
            }
        }
        rcs.gates.push(Gate {
            a: x.lc.0,
            x: x.lc.1,
            b: y.lc.0,
            y: y.lc.1,
            c: -z.lc.0,
            z: z.lc.1,
            d: Num::ZERO,
            e: x.lc.2 + y.lc.2 - z.lc.2,
        })
    }

    pub fn inputize(n: &CNum<Fr>) {
        let v = if n.lc.0 == Num::ONE && n.lc.2 == Num::ZERO {
            n.lc.1
        } else {
            let m: CNum<Fr> = n.derive_alloc(n.value.as_ref());
            m.assert_eq(n);
            m.lc.1
        };

        n.get_cs().borrow_mut().public.push(v);
    }

    pub fn alloc(cs: &RCS<Fr>, value: Option<&Num<Fr>>) -> CNum<Fr> {
        let mut rcs = cs.borrow_mut();
        let n_vars = rcs.values.len();
        let v = n_vars;
        rcs.values.push(value.cloned());
        CNum {
            value: value.cloned(),
            lc: (Num::ONE, v, Num::ZERO),
            cs: cs.clone(),
        }
    }
}
