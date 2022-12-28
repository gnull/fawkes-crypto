use crate::{
    circuit::num::CNum,
    core::signal::Signal,
    ff_uint::{Num, PrimeField},
};

use std::{cell::RefCell, rc::Rc};

pub type RCS<C> = Rc<RefCell<C>>;

#[derive(Clone, Debug)]
pub enum Gate<Fr: PrimeField> {
    // a*x + b *y + c*z + d*x*y + e == 0
    Arith(
        Num<Fr>,
        usize,
        Num<Fr>,
        usize,
        Num<Fr>,
        usize,
        Num<Fr>,
        Num<Fr>,
    ),
}

pub trait CS: Clone {
    type Fr: PrimeField;
    type GateIterator: Iterator<Item=Gate<Self::Fr>>;

    fn num_gates(&self) -> usize;
    fn num_input(&self) -> usize;
    fn num_aux(&self) -> usize;
    fn get_value(&self, index: usize) -> Option<Num<Self::Fr>>;
    fn get_gate_iterator(&self) -> Self::GateIterator;

    // a * b == c
    fn enforce_mul(a: &CNum<Self>, b: &CNum<Self>, c: &CNum<Self>);

    // a + b == c
    fn enforce_add(a: &CNum<Self>, b: &CNum<Self>, c: &CNum<Self>);

    fn inputize(n: &CNum<Self>);
    fn alloc(cs: &RCS<Self>, value: Option<&Num<Self::Fr>>) -> CNum<Self>;

    fn const_tracker_before(&mut self) -> Option<bool> {
        None
    }

    fn const_tracker_after(&mut self, _:bool) {}
}

#[derive(Clone, Debug)]
pub struct BuildCS<Fr: PrimeField> {
    pub values: Vec<Option<Num<Fr>>>,
    pub gates: Vec<Gate<Fr>>,
    pub tracking: bool,
    pub public: Vec<usize>,
}

impl<Fr: PrimeField> BuildCS<Fr> {
    pub fn new(tracking: bool) -> Self {
        Self {
            values: vec![],
            gates: vec![],
            tracking,
            public: vec![],
        }
    }

    pub fn rc_new(tracking: bool) -> RCS<Self> {
        Rc::new(RefCell::new(Self::new(tracking)))
    }
}

impl<Fr: PrimeField> CS for BuildCS<Fr> {
    type Fr = Fr;
    type GateIterator = std::vec::IntoIter<Gate<Self::Fr>>;

    fn num_gates(&self) -> usize {
        self.gates.len()
    }

    fn num_input(&self) -> usize {
        self.public.len()
    }

    fn num_aux(&self) -> usize {
        self.values.len() - self.public.len()
    }

    fn get_value(&self, index: usize) -> Option<Num<Self::Fr>> {
        self.values[index]
    }

    fn get_gate_iterator(&self) -> Self::GateIterator {
        self.gates.clone().into_iter()
    }

    // a*b === c
    fn enforce_mul(a: &CNum<Self>, b: &CNum<Self>, c: &CNum<Self>) {
        let mut rcs = a.get_cs().borrow_mut();
        if rcs.tracking {
            match (a.value, b.value, c.value) {
                (Some(a), Some(b), Some(c)) => {
                    assert!(a * b == c, "Not satisfied constraint");
                }
                _ => {}
            }
        }
        rcs.gates.push(Gate::Arith(
            a.lc.0 * b.lc.2,
            a.lc.1,
            a.lc.2 * b.lc.0,
            b.lc.1,
            -c.lc.0,
            c.lc.1,
            a.lc.0 * b.lc.0,
            a.lc.2 * b.lc.2 - c.lc.2,
        ))
    }

    fn enforce_add(a: &CNum<Self>, b: &CNum<Self>, c: &CNum<Self>) {
        let mut rcs = a.get_cs().borrow_mut();
        if rcs.tracking {
            match (a.value, b.value, c.value) {
                (Some(a), Some(b), Some(c)) => {
                    assert!(a + b == c, "Not satisfied constraint");
                }
                _ => {}
            }
        }
        rcs.gates.push(Gate::Arith(
            a.lc.0,
            a.lc.1,
            b.lc.0,
            b.lc.1,
            -c.lc.0,
            c.lc.1,
            Num::ZERO,
            a.lc.2 + b.lc.2 - c.lc.2,
        ))
    }

    fn inputize(n: &CNum<Self>) {
        let v = if n.lc.0 == Num::ONE && n.lc.2 == Num::ZERO {
            n.lc.1
        } else {
            let m: CNum<Self> = n.derive_alloc(n.value.as_ref());
            m.assert_eq(n);
            m.lc.1
        };

        n.get_cs().borrow_mut().public.push(v);
    }

    fn alloc(cs: &RCS<Self>, value: Option<&Num<Fr>>) -> CNum<Self> {
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
