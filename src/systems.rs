use std::{any::TypeId, collections::HashSet, rc::Rc};

use crate::{query::WorldQuery, World};

#[derive(Clone, Default)]
pub struct SystemStage<'a> {
    pub systems: Vec<ErasedSystem<'a, ()>>,
}

impl<'a> SystemStage<'a> {
    pub fn with_system<S, P>(mut self, system: S) -> Self
    where
        S: IntoSystem<'a, P, ()>,
    {
        self.systems.push(system.system());
        self
    }
}

pub type InnerSystem<'a, R> = Box<dyn Fn(&'a World) -> R + 'a>;

pub struct ErasedSystem<'a, R> {
    pub(crate) execute: InnerSystem<'a, R>,
    pub(crate) components_mut: fn() -> HashSet<TypeId>,
    pub(crate) resources_mut: fn() -> HashSet<TypeId>,
    pub(crate) components_const: fn() -> HashSet<TypeId>,
    pub(crate) resources_const: fn() -> HashSet<TypeId>,
    factory: Rc<dyn Fn() -> InnerSystem<'a, R>>,
}

unsafe impl<R> Send for ErasedSystem<'_, R> {}
unsafe impl<R> Sync for ErasedSystem<'_, R> {}

impl<'a, R> Clone for ErasedSystem<'a, R> {
    fn clone(&self) -> Self {
        Self {
            execute: (self.factory)(),
            components_mut: self.components_mut,
            resources_mut: self.resources_mut,
            components_const: self.components_const,
            resources_const: self.resources_const,
            factory: self.factory.clone(),
        }
    }
}

pub trait IntoSystem<'a, Param, R> {
    fn system(self) -> ErasedSystem<'a, R>;
}

macro_rules! impl_intosys_fn {
    ($($t: ident),* $(,)*) => {
        impl<'a, R, F, $($t: WorldQuery<'a> + 'static,)*>
            IntoSystem<'a, ($($t),*,), R> for F
        where
            F: Fn($($t),*) -> R + 'static + Copy,
        {
            fn system(self) -> ErasedSystem<'a, R> {
                let factory: Rc<dyn Fn()-> InnerSystem<'a, R>>
                    = Rc::new(move || {
                        Box::new(move |world: &'a World| {
                            (self)(
                                $(<$t>::new(world),)*
                            )
                        })
                    });
                ErasedSystem {
                    execute: factory(),
                    components_mut: || {
                        let mut res = HashSet::new();
                        $(<$t>::components_mut(&mut res);)*
                        res
                    },
                    resources_mut: || {
                        let mut res = HashSet::new();
                        $(<$t>::resources_mut(&mut res);)*
                        res
                    },
                    components_const: || {
                        let mut res = HashSet::new();
                        $(<$t>::components_const(&mut res);)*
                        res
                    },
                    resources_const: || {
                        let mut res = HashSet::new();
                        $(<$t>::resources_const(&mut res);)*
                        res
                    },
                    factory,
                }
            }
        }
    };
}

impl_intosys_fn!(Q0);
impl_intosys_fn!(Q0, Q1);
impl_intosys_fn!(Q0, Q1, Q2);
impl_intosys_fn!(Q0, Q1, Q2, Q3);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13, Q14);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13, Q14, Q15);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13, Q14, Q15, Q16);
impl_intosys_fn!(Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13, Q14, Q15, Q16, Q17);
impl_intosys_fn!(
    Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13, Q14, Q15, Q16, Q17, Q18
);
impl_intosys_fn!(
    Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13, Q14, Q15, Q16, Q17, Q18, Q19
);
impl_intosys_fn!(
    Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q13, Q14, Q15, Q16, Q17, Q18, Q19, Q20
);
