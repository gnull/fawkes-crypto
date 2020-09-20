pub trait Signal: Sized+Clone {
    type Value: Clone + Sized;

    fn as_const(&self) -> Option<Self::Value>;

    fn get_value(&self) -> Option<Self::Value>;

    // fn get_cs(&self) -> &'a CS;

    // fn from_const(cs:&'a CS, value: &Self::Value) -> Self;

    // fn alloc(cs:&'a CS, value:Option<&Self::Value>) -> Self;

    // fn switch(&self, bit: &CBool<'a, CS>, if_else: &Self) -> Self;

    // fn assert_const(&self, value: &Self::Value);

    // fn assert_eq(&self, other:&Self);

    // fn is_eq(&self, other:&Self) -> CBool<'a, CS>;

    // fn inputize(&self);

    // fn linearize_builder(&self, acc: &mut Vec<CNum<'a, CS>>);

    // fn linearize(&self) -> Vec<CNum<'a, CS>> {
    //     let mut acc = Vec::new();
    //     self.linearize_builder(&mut acc);
    //     acc
    // }

    // #[inline]
    // fn derive_const<T:Signal<'a, CS>>(&self, value: &T::Value) -> T {
    //     T::from_const(self.get_cs(), value)
    // }

    // #[inline]
    // fn derive_alloc<T:Signal<'a, CS>>(&self, value:Option<&T::Value>) -> T {
    //     T::alloc(self.get_cs(), value)
    // }
}