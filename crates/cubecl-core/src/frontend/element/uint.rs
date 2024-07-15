use crate::frontend::{CubeContext, CubePrimitive, CubeType, ExpandElement, Numeric};
use crate::ir::{Elem, Item, Variable, Vectorization};
use crate::prelude::{index_assign, KernelBuilder, KernelLauncher};
use crate::{frontend::Comptime, Runtime};

use super::{
    init_expand_element, ExpandElementBaseInit, ExpandElementTyped, LaunchArgExpand,
    ScalarArgSettings, Vectorized,
};

#[derive(Clone, Copy, Debug)]
/// An unsigned int.
/// Preferred for indexing operations
pub struct UInt {
    pub val: u32,
    pub vectorization: u8,
}

impl CubeType for UInt {
    type ExpandType = ExpandElementTyped<Self>;
}

impl ExpandElementBaseInit for UInt {
    fn init_elem(context: &mut CubeContext, elem: ExpandElement) -> ExpandElement {
        init_expand_element(context, elem)
    }
}

impl CubePrimitive for UInt {
    fn as_elem() -> Elem {
        Elem::UInt
    }
}

// For use within comptime.
impl Into<<UInt as CubeType>::ExpandType> for UInt {
    fn into(self) -> ExpandElementTyped<UInt> {
        let elem: ExpandElement = self.into();
        elem.into()
    }
}

impl LaunchArgExpand for UInt {
    fn expand(
        builder: &mut KernelBuilder,
        vectorization: Vectorization,
    ) -> ExpandElementTyped<Self> {
        assert_eq!(vectorization, 1, "Attempted to vectorize a scalar");
        builder.scalar(UInt::as_elem()).into()
    }
}

impl ScalarArgSettings for u32 {
    fn register<R: Runtime>(&self, settings: &mut KernelLauncher<R>) {
        settings.register_u32(*self);
    }
}

impl Numeric for UInt {
    type Primitive = u32;
}

impl UInt {
    pub const fn new(val: u32) -> Self {
        Self {
            val,
            vectorization: 1,
        }
    }

    pub fn __expand_new(_context: &mut CubeContext, val: u32) -> <Self as CubeType>::ExpandType {
        let new_var = Variable::ConstantScalar {
            value: val as f64,
            elem: Self::as_elem(),
        };
        ExpandElement::Plain(new_var).into()
    }

    pub fn vectorized(val: u32, vectorization: UInt) -> Self {
        if vectorization.val == 1 {
            Self::new(val)
        } else {
            Self {
                val,
                vectorization: vectorization.val as u8,
            }
        }
    }

    pub fn __expand_vectorized(
        context: &mut CubeContext,
        val: u32,
        vectorization: UInt,
    ) -> <Self as CubeType>::ExpandType {
        if vectorization.val == 1 {
            Self::__expand_new(context, val)
        } else {
            let mut new_var =
                context.create_local(Item::vectorized(Self::as_elem(), vectorization.val as u8));
            for (i, element) in vec![val; vectorization.val as usize].iter().enumerate() {
                new_var = index_assign::expand(context, new_var, i, *element);
            }

            new_var.into()
        }
    }
}

impl From<u32> for UInt {
    fn from(value: u32) -> Self {
        UInt::new(value)
    }
}

impl From<Comptime<u32>> for UInt {
    fn from(value: Comptime<u32>) -> Self {
        UInt::new(value.inner)
    }
}

impl From<usize> for UInt {
    fn from(value: usize) -> Self {
        UInt::new(value as u32)
    }
}

impl From<i32> for UInt {
    fn from(value: i32) -> Self {
        UInt::new(value as u32)
    }
}

impl Vectorized for UInt {
    fn vectorization_factor(&self) -> UInt {
        UInt {
            val: self.vectorization as u32,
            vectorization: 1,
        }
    }

    fn vectorize(mut self, factor: UInt) -> Self {
        self.vectorization = factor.vectorization;
        self
    }
}
