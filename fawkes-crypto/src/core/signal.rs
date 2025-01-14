use crate::{
    circuit::{bool::CBool, cs::{CS, RCS}},
    core::sizedvec::SizedVec,
};
use impl_trait_for_tuples::impl_for_tuples;

pub use fawkes_crypto_derive::Signal;

pub trait Signal<C: CS>: Sized + Clone {
    type Value: Clone + Sized;

    /// Return Some(value) if signal is constant. Otherwise, return None
    fn as_const(&self) -> Option<Self::Value>;

    /// Return value if we apply the data to the circuit. Otherwise, return None
    fn get_value(&self) -> Option<Self::Value>;

    #[inline]
    fn derive_const<T: Signal<C>>(&self, value: &T::Value) -> T {
        T::from_const(self.get_cs(), value)
    }

    /// Create a new signal with constant value (for R1CS case this is linear compination with only 1st nonzero element)
    fn from_const(cs: &RCS<C>, value: &Self::Value) -> Self;

    /// Get link to the circut
    fn get_cs(&self) -> &RCS<C>;

    /// Create a new signal with custom value
    fn alloc(cs: &RCS<C>, value: Option<&Self::Value>) -> Self;

    /// Returns self if bit is true, if_else otherwise
    fn switch(&self, bit: &CBool<C>, if_else: &Self) -> Self;

    /// And constant constraint to the circuit (for R1CS case it is self * ONE == value)
    fn assert_const(&self, value: &Self::Value);

    /// Add eq constraint to the circuit
    fn assert_eq(&self, other: &Self);

    /// Return true if values are equal, false otherwise
    fn is_eq(&self, other: &Self) -> CBool<C>;

    /// Make the signal public
    fn inputize(&self);

    #[inline]
    fn derive_alloc<T: Signal<C>>(&self, value: Option<&T::Value>) -> T {
        T::alloc(self.get_cs(), value)
    }
}

impl<C: CS, T: Signal<C>, const L: usize> Signal<C> for SizedVec<T, L> {
    type Value = SizedVec<T::Value, L>;

    fn get_value(&self) -> Option<Self::Value> {
        self.iter().map(|v| v.get_value()).collect()
    }

    fn switch(&self, bit: &CBool<C>, if_else: &Self) -> Self {
        self.iter()
            .zip(if_else.iter())
            .map(|(t, f)| t.switch(bit, f))
            .collect()
    }

    fn get_cs(&self) -> &RCS<C> {
        self[0].get_cs()
    }

    fn from_const(cs: &RCS<C>, value: &Self::Value) -> Self {
        value.iter().map(|v| T::from_const(cs, v)).collect()
    }

    fn as_const(&self) -> Option<Self::Value> {
        self.iter().map(|v| v.as_const()).collect()
    }

    fn alloc(cs: &RCS<C>, value: Option<&Self::Value>) -> Self {
        match value {
            Some(value) => value.iter().map(|v| T::alloc(cs, Some(v))).collect(),
            _ => (0..L).map(|_| T::alloc(cs, None)).collect(),
        }
    }

    fn assert_const(&self, value: &Self::Value) {
        self.iter()
            .zip(value.iter())
            .for_each(|(s, v)| s.assert_const(v));
    }

    fn inputize(&self) {
        self.iter().for_each(|s| s.inputize());
    }

    fn assert_eq(&self, other: &Self) {
        self.iter()
            .zip(other.iter())
            .for_each(|(s, o)| s.assert_eq(o));
    }

    fn is_eq(&self, other: &Self) -> CBool<C> {
        let mut acc = self.derive_const(&true);
        for i in 0..L {
            acc &= self[i].is_eq(&other[i]);
        }
        acc
    }
}

#[impl_for_tuples(1, 17)]
impl<C: CS> Signal<C> for Tuple {
    for_tuples!( type Value = ( #( Tuple::Value ),* ); );

    fn get_value(&self) -> Option<Self::Value> {
        Some((for_tuples!( #( self.Tuple.get_value()?),* )))
    }

    fn switch(&self, bit: &CBool<C>, if_else: &Self) -> Self {
        (for_tuples!( #(self.Tuple.switch(bit, &if_else.Tuple) ),* ))
    }

    fn get_cs(&self) -> &RCS<C> {
        self.0.get_cs()
    }

    fn from_const(cs: &RCS<C>, value: &Self::Value) -> Self {
        (for_tuples!( #( Tuple::from_const(cs, &value.Tuple)),* ))
    }

    fn as_const(&self) -> Option<Self::Value> {
        Some((for_tuples!( #( self.Tuple.as_const()?),* )))
    }

    fn alloc(cs: &RCS<C>, value: Option<&Self::Value>) -> Self {
        match value {
            Some(value) => (for_tuples!( #( Tuple::alloc(cs, Some(&value.Tuple) )),* )),
            _ => (for_tuples!( #( Tuple::alloc(cs, None)),* )),
        }
    }

    fn assert_const(&self, value: &Self::Value) {
        for_tuples!( #(self.Tuple.assert_const(&value.Tuple); )* );
    }

    fn inputize(&self) {
        for_tuples!( #(self.Tuple.inputize(); )* );
    }

    fn assert_eq(&self, other: &Self) {
        for_tuples!( #(self.Tuple.assert_eq(&other.Tuple); )* );
    }

    fn is_eq(&self, other: &Self) -> CBool<C> {
        let mut acc = self.derive_const(&true);
        for_tuples!( #(acc &= self.Tuple.is_eq(&other.Tuple); )* );
        acc
    }
}
