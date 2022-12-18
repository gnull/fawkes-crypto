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

    // a*b === c
    pub fn enforce_mul(a: &CNum<Fr>, b: &CNum<Fr>, c: &CNum<Fr>) {
        let mut rcs = a.get_cs().borrow_mut();
        if rcs.tracking {
            match (a.value, b.value, c.value) {
                (Some(a), Some(b), Some(c)) => {
                    assert!(a * b == c, "Not satisfied constraint");
                }
                _ => {}
            }
        }
        rcs.gates.push(Gate {
            a: a.lc.0 * b.lc.2,
            x: a.lc.1,
            b: a.lc.2 * b.lc.0,
            y: b.lc.1,
            c: -c.lc.0,
            z: c.lc.1,
            d: a.lc.0 * b.lc.0,
            e: a.lc.2 * b.lc.2 - c.lc.2,
        })
    }

    pub fn enforce_add(a: &CNum<Fr>, b: &CNum<Fr>, c: &CNum<Fr>) {
        let mut rcs = a.get_cs().borrow_mut();
        if rcs.tracking {
            match (a.value, b.value, c.value) {
                (Some(a), Some(b), Some(c)) => {
                    assert!(a + b == c, "Not satisfied constraint");
                }
                _ => {}
            }
        }
        rcs.gates.push(Gate {
            a: a.lc.0,
            x: a.lc.1,
            b: b.lc.0,
            y: b.lc.1,
            c: -c.lc.0,
            z: c.lc.1,
            d: Num::ZERO,
            e: a.lc.2 + b.lc.2 - c.lc.2,
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
