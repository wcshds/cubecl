use std::ops::Deref;

use crate::frontend::{CubeContext, ExpandElement, UInt};
use crate::ir::{Branch, Elem, If, IfElse, Item, Loop, RangeLoop, Variable};

use super::comptime::Comptime;
use super::ExpandElementTyped;

/// UInt range. Equivalent to:
/// ```no_run
/// for i in start..end { ... }
/// ```
pub fn range<S, E>(start: S, end: E, _unroll: Comptime<bool>) -> impl Iterator<Item = UInt>
where
    S: Into<UInt>,
    E: Into<UInt>,
{
    let start: UInt = start.into();
    let end: UInt = end.into();

    (start.val..end.val).map(UInt::new)
}

/// Stepped range. Equivalent to:
/// ```no_run
/// for i in (start..end).step_by(step) { ... }
/// ```
pub fn range_stepped<S, E, Step>(
    start: S,
    end: E,
    step: Step,
    _unroll: Comptime<bool>,
) -> impl Iterator<Item = UInt>
where
    S: Into<UInt>,
    E: Into<UInt>,
    Step: Into<UInt>,
{
    let start: UInt = start.into();
    let end: UInt = end.into();
    let step: UInt = step.into();

    (start.val..end.val)
        .step_by(step.val as usize)
        .map(UInt::new)
}

pub fn range_expand<F, S, E>(context: &mut CubeContext, start: S, end: E, unroll: bool, mut func: F)
where
    F: FnMut(&mut CubeContext, ExpandElementTyped<UInt>),
    S: Into<ExpandElementTyped<UInt>>,
    E: Into<ExpandElementTyped<UInt>>,
{
    let start: ExpandElementTyped<UInt> = start.into();
    let end: ExpandElementTyped<UInt> = end.into();
    let start = start.expand;
    let end = end.expand;

    if unroll {
        let start = match start.deref() {
            Variable::ConstantScalar(value) => value.as_usize(),
            _ => panic!("Only constant start can be unrolled."),
        };
        let end = match end.deref() {
            Variable::ConstantScalar(value) => value.as_usize(),
            _ => panic!("Only constant end can be unrolled."),
        };

        for i in start..end {
            let var: ExpandElement = i.into();
            func(context, var.into())
        }
    } else {
        let mut child = context.child();
        let index_ty = Item::new(Elem::UInt);
        let i = child.scope.borrow_mut().create_local_undeclared(index_ty);
        let i = ExpandElement::Plain(i);

        func(&mut child, i.clone().into());

        context.register(Branch::RangeLoop(RangeLoop {
            i: *i,
            start: *start,
            end: *end,
            step: None,
            scope: child.into_scope(),
        }));
    }
}

pub fn range_stepped_expand<F, S, E, Step>(
    context: &mut CubeContext,
    start: S,
    end: E,
    step: Step,
    unroll: bool,
    mut func: F,
) where
    F: FnMut(&mut CubeContext, ExpandElementTyped<UInt>),
    S: Into<ExpandElementTyped<UInt>>,
    E: Into<ExpandElementTyped<UInt>>,
    Step: Into<ExpandElementTyped<UInt>>,
{
    let start: ExpandElementTyped<UInt> = start.into();
    let end: ExpandElementTyped<UInt> = end.into();
    let step: ExpandElementTyped<UInt> = step.into();
    let start = start.expand;
    let end = end.expand;
    let step = step.expand;

    if unroll {
        let start = match start.deref() {
            Variable::ConstantScalar(value) => value.as_usize(),
            _ => panic!("Only constant start can be unrolled."),
        };
        let end = match end.deref() {
            Variable::ConstantScalar(value) => value.as_usize(),
            _ => panic!("Only constant end can be unrolled."),
        };
        let step: usize = match step.deref() {
            Variable::ConstantScalar(value) => value.as_usize(),
            _ => panic!("Only constant step can be unrolled."),
        };

        for i in (start..end).step_by(step) {
            let var: ExpandElement = i.into();
            func(context, var.into())
        }
    } else {
        let mut child = context.child();
        let index_ty = Item::new(Elem::UInt);
        let i = child.scope.borrow_mut().create_local_undeclared(index_ty);
        let i = ExpandElement::Plain(i);

        func(&mut child, i.clone().into());

        context.register(Branch::RangeLoop(RangeLoop {
            i: *i,
            start: *start,
            end: *end,
            step: Some(*step),
            scope: child.into_scope(),
        }));
    }
}

pub fn if_expand<IF>(
    context: &mut CubeContext,
    comptime_cond: Option<bool>,
    runtime_cond: ExpandElement,
    mut block: IF,
) where
    IF: FnMut(&mut CubeContext),
{
    match comptime_cond {
        Some(cond) => {
            if cond {
                block(context);
            }
        }
        None => {
            let mut child = context.child();

            block(&mut child);

            context.register(Branch::If(If {
                cond: *runtime_cond,
                scope: child.into_scope(),
            }));
        }
    }
}

pub fn if_else_expand<IF, EL>(
    context: &mut CubeContext,
    comptime_cond: Option<bool>,
    runtime_cond: ExpandElement,
    mut then_block: IF,
    mut else_block: EL,
) where
    IF: FnMut(&mut CubeContext),
    EL: FnMut(&mut CubeContext),
{
    match comptime_cond {
        Some(cond) => {
            if cond {
                then_block(context);
            } else {
                else_block(context);
            }
        }
        None => {
            let mut then_child = context.child();
            then_block(&mut then_child);

            let mut else_child = context.child();
            else_block(&mut else_child);

            context.register(Branch::IfElse(IfElse {
                cond: *runtime_cond,
                scope_if: then_child.into_scope(),
                scope_else: else_child.into_scope(),
            }));
        }
    }
}

pub fn break_expand(context: &mut CubeContext) {
    context.register(Branch::Break);
}

pub fn return_expand(context: &mut CubeContext) {
    context.register(Branch::Return);
}

pub fn loop_expand<FB>(context: &mut CubeContext, mut block: FB)
where
    FB: FnMut(&mut CubeContext),
{
    let mut inside_loop = context.child();

    block(&mut inside_loop);
    context.register(Branch::Loop(Loop {
        scope: inside_loop.into_scope(),
    }));
}

pub fn while_loop_expand<FC, FB>(context: &mut CubeContext, mut cond_fn: FC, mut block: FB)
where
    FC: FnMut(&mut CubeContext) -> ExpandElementTyped<bool>,
    FB: FnMut(&mut CubeContext),
{
    let mut inside_loop = context.child();

    let cond: ExpandElement = cond_fn(&mut inside_loop).into();
    if_expand(&mut inside_loop, None, cond, break_expand);

    block(&mut inside_loop);
    context.register(Branch::Loop(Loop {
        scope: inside_loop.into_scope(),
    }));
}
