use super::{
    base::{Item, Variable},
    Elem, IndexedVariable, Subgroup,
};
use std::fmt::Display;

/// All instructions that can be used in a WGSL compute shader.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Some variants might not be used with different flags
pub enum Instruction {
    DeclareVariable {
        var: Variable,
    },
    Max {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Min {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Add {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Fma {
        a: Variable,
        b: Variable,
        c: Variable,
        out: Variable,
    },
    If {
        cond: Variable,
        instructions: Vec<Instruction>,
    },
    IfElse {
        cond: Variable,
        instructions_if: Vec<Instruction>,
        instructions_else: Vec<Instruction>,
    },
    Return,
    Break,
    WorkgroupBarrier,
    StorageBarrier,
    // Index handles casting to correct local variable.
    Index {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    // Index assign handles casting to correct output variable.
    IndexAssign {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    // Assign handle casting to correct output variable.
    Assign {
        input: Variable,
        out: Variable,
    },
    Modulo {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Sub {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Mul {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Div {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Abs {
        input: Variable,
        out: Variable,
    },
    Exp {
        input: Variable,
        out: Variable,
    },
    Log {
        input: Variable,
        out: Variable,
    },
    Log1p {
        input: Variable,
        out: Variable,
    },
    Cos {
        input: Variable,
        out: Variable,
    },
    Sin {
        input: Variable,
        out: Variable,
    },
    Tanh {
        input: Variable,
        out: Variable,
    },
    Powf {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Sqrt {
        input: Variable,
        out: Variable,
    },
    Erf {
        input: Variable,
        out: Variable,
    },
    Recip {
        input: Variable,
        out: Variable,
    },
    Equal {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Lower {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Clamp {
        input: Variable,
        min_value: Variable,
        max_value: Variable,
        out: Variable,
    },
    Greater {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    LowerEqual {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    GreaterEqual {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    NotEqual {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Stride {
        dim: Variable,
        position: usize,
        out: Variable,
    },
    Length {
        var: Variable,
        out: Variable,
    },
    Shape {
        dim: Variable,
        position: usize,
        out: Variable,
    },
    RangeLoop {
        i: Variable,
        start: Variable,
        end: Variable,
        step: Option<Variable>,
        instructions: Vec<Instruction>,
    },
    And {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Or {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Not {
        input: Variable,
        out: Variable,
    },
    Loop {
        instructions: Vec<Instruction>,
    },
    BitwiseAnd {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    BitwiseXor {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    ShiftLeft {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    ShiftRight {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Floor {
        input: Variable,
        out: Variable,
    },
    Ceil {
        input: Variable,
        out: Variable,
    },
    Remainder {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Slice {
        input: Variable,
        start: Variable,
        end: Variable,
        out: Variable,
    },
    Bitcast {
        input: Variable,
        out: Variable,
    },
    AtomicLoad {
        input: Variable,
        out: Variable,
    },
    AtomicStore {
        input: Variable,
        out: Variable,
    },
    AtomicSwap {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    AtomicCompareExchangeWeak {
        lhs: Variable,
        cmp: Variable,
        value: Variable,
        out: Variable,
    },
    AtomicAdd {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    AtomicSub {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    AtomicMax {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    AtomicMin {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    AtomicAnd {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    AtomicOr {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    AtomicXor {
        lhs: Variable,
        rhs: Variable,
        out: Variable,
    },
    Subgroup(Subgroup),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::DeclareVariable { var } => {
                let item = var.item();
                f.write_fmt(format_args!("var {var}: {item};\n"))
            }
            Instruction::Add { lhs, rhs, out } => {
                if out.is_atomic() {
                    assert_eq!(lhs, out, "Can't use regular addition on atomic");
                    f.write_fmt(format_args!("atomicAdd({out}, {rhs});\n"))
                } else {
                    f.write_fmt(format_args!("{out} = {lhs} + {rhs};\n"))
                }
            }
            Instruction::Slice {
                input,
                start,
                end,
                out,
            } => {
                f.write_fmt(format_args!("let {out}_offset = {start};\n"))?;
                f.write_fmt(format_args!("let {out}_length = {end} - {start};\n"))?;
                f.write_fmt(format_args!("let {out}_ptr = &{input};\n"))
            }
            Instruction::Fma { a, b, c, out } => {
                f.write_fmt(format_args!("{out} = fma({a}, {b}, {c});\n"))
            }
            Instruction::Min { lhs, rhs, out } => {
                if out.is_atomic() {
                    assert_eq!(lhs, out, "Can't use regular min on atomic");
                    f.write_fmt(format_args!("atomicMin({out}, {rhs});\n"))
                } else {
                    f.write_fmt(format_args!("{out} = min({lhs}, {rhs});\n"))
                }
            }
            Instruction::Max { lhs, rhs, out } => {
                if out.is_atomic() {
                    assert_eq!(lhs, out, "Can't use regular max on atomic");
                    f.write_fmt(format_args!("atomicMax({out}, {rhs});\n"))
                } else {
                    f.write_fmt(format_args!("{out} = max({lhs}, {rhs});\n"))
                }
            }
            Instruction::And { lhs, rhs, out } => {
                if out.is_atomic() {
                    assert_eq!(lhs, out, "Can't use regular and on atomic");
                    f.write_fmt(format_args!("atomicAnd({out}, {rhs});\n"))
                } else {
                    f.write_fmt(format_args!("{out} = {lhs} && {rhs};\n"))
                }
            }
            Instruction::Or { lhs, rhs, out } => {
                if out.is_atomic() {
                    assert_eq!(lhs, out, "Can't use regular or on atomic");
                    f.write_fmt(format_args!("atomicOr({out}, {rhs});\n"))
                } else {
                    f.write_fmt(format_args!("{out} = {lhs} || {rhs};\n"))
                }
            }
            Instruction::Not { input, out } => f.write_fmt(format_args!("{out} = !{input};\n")),
            Instruction::Index { lhs, rhs, out } => match lhs {
                Variable::Slice { item, .. } => {
                    let offset = Variable::Named {
                        name: format!("{lhs}_offset"),
                        item: Item::Scalar(Elem::U32),
                        is_array: false,
                    };
                    let lhs = Variable::Named {
                        name: format!("(*{lhs}_ptr)"),
                        item: *item,
                        is_array: true,
                    };
                    index(f, &lhs, rhs, out, Some(offset))
                }
                _ => index(f, lhs, rhs, out, None),
            },
            Instruction::Modulo { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = {lhs} % {rhs};\n"))
            }
            Instruction::Remainder { lhs, rhs, out } => f.write_fmt(format_args!(
                "{out} = {lhs} - {rhs} * floor({lhs} / {rhs});\n"
            )),
            Instruction::Sub { lhs, rhs, out } => {
                if out.is_atomic() {
                    assert_eq!(lhs, out, "Can't use regular sub on atomic");
                    f.write_fmt(format_args!("atomicSub({out}, {rhs});\n"))
                } else {
                    f.write_fmt(format_args!("{out} = {lhs} - {rhs};\n"))
                }
            }
            Instruction::Mul { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = {lhs} * {rhs};\n"))
            }
            Instruction::Div { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = {lhs} / {rhs};\n"))
            }
            Instruction::Abs { input, out } => f.write_fmt(format_args!("{out} = abs({input});\n")),
            Instruction::Exp { input, out } => f.write_fmt(format_args!("{out} = exp({input});\n")),
            Instruction::Log { input, out } => f.write_fmt(format_args!("{out} = log({input});\n")),
            Instruction::Clamp {
                input,
                min_value,
                max_value,
                out,
            } => unroll(
                f,
                out.item().vectorization_factor(),
                [input, min_value, max_value, out],
                |f, [input, min, max, out]| {
                    f.write_fmt(format_args!("{out} = clamp({input}, {min}, {max});\n"))
                },
            ),
            Instruction::Powf { lhs, rhs, out } => {
                let vectorization_factor = out.item().vectorization_factor();

                if rhs.is_always_scalar() {
                    f.write_fmt(format_args!("{out} = powf_scalar({lhs}, {rhs});\n"))
                } else {
                    unroll(
                        f,
                        vectorization_factor,
                        [lhs, rhs, out],
                        |f, [lhs, rhs, out]| {
                            f.write_fmt(format_args!("{out} = powf_primitive({lhs}, {rhs});\n"))
                        },
                    )
                }
            }
            Instruction::Sqrt { input, out } => {
                f.write_fmt(format_args!("{out} = sqrt({input});\n"))
            }
            Instruction::Log1p { input, out } => {
                f.write_fmt(format_args!("{out} = log({input} + 1.0);\n"))
            }
            Instruction::Cos { input, out } => f.write_fmt(format_args!("{out} = cos({input});\n")),
            Instruction::Sin { input, out } => f.write_fmt(format_args!("{out} = sin({input});\n")),
            Instruction::Tanh { input, out } => {
                #[cfg(target_os = "macos")]
                let result = f.write_fmt(format_args!("{out} = safe_tanh({input});\n"));
                #[cfg(not(target_os = "macos"))]
                let result = f.write_fmt(format_args!("{out} = tanh({input});\n"));

                result
            }
            Instruction::Erf { input, out } => f.write_fmt(format_args!("{out} = erf({input});\n")),
            Instruction::Recip { input, out } => {
                f.write_fmt(format_args!("{out} = 1.0 / {input};"))
            }
            Instruction::Equal { lhs, rhs, out } => comparison(lhs, rhs, out, "==", f),
            Instruction::Lower { lhs, rhs, out } => comparison(lhs, rhs, out, "<", f),
            Instruction::Greater { lhs, rhs, out } => comparison(lhs, rhs, out, ">", f),
            Instruction::LowerEqual { lhs, rhs, out } => comparison(lhs, rhs, out, "<=", f),
            Instruction::GreaterEqual { lhs, rhs, out } => comparison(lhs, rhs, out, ">=", f),
            Instruction::NotEqual { lhs, rhs, out } => comparison(lhs, rhs, out, "!=", f),
            Instruction::Assign { input, out } => match out.item() {
                Item::Vec4(elem) => {
                    let input0 = input.index(0);
                    let input1 = input.index(1);
                    let input2 = input.index(2);
                    let input3 = input.index(3);

                    f.write_fmt(format_args!(
                        "{out} = vec4(
    {elem}({input0}),
    {elem}({input1}),
    {elem}({input2}),
    {elem}({input3}),
);
"
                    ))
                }
                Item::Vec3(elem) => {
                    let input0 = input.index(0);
                    let input1 = input.index(1);
                    let input2 = input.index(2);

                    f.write_fmt(format_args!(
                        "{out} = vec3(
    {elem}({input0}),
    {elem}({input1}),
    {elem}({input2}),
);
"
                    ))
                }
                Item::Vec2(elem) => {
                    let input0 = input.index(0);
                    let input1 = input.index(1);

                    f.write_fmt(format_args!(
                        "{out} = vec2(
    {elem}({input0}),
    {elem}({input1}),
);
"
                    ))
                }
                Item::Scalar(elem) => {
                    if elem.is_atomic() {
                        f.write_fmt(format_args!("let {out} = &{input};\n"))
                    } else {
                        f.write_fmt(format_args!("{out} = {elem}({input});\n"))
                    }
                }
            },
            Instruction::Stride { dim, position, out } => f.write_fmt(format_args!(
                "{out} = info[({position}u * rank_2) + {dim} + 1u];\n"
            )),
            Instruction::Shape { dim, position, out } => f.write_fmt(format_args!(
                "{out} = info[({position}u * rank_2) + rank + {dim} + 1u];\n"
            )),
            Instruction::RangeLoop {
                i,
                start,
                end,
                step,
                instructions,
            } => {
                let increment = step
                    .as_ref()
                    .map(|step| format!("{i} += {step}"))
                    .unwrap_or_else(|| format!("{i}++"));

                f.write_fmt(format_args!(
                    "
for (var {i}: u32 = {start}; {i} < {end}; {increment}) {{
"
                ))?;
                for instruction in instructions {
                    f.write_fmt(format_args!("{instruction}"))?;
                }

                f.write_str("}\n")
            }
            Instruction::IndexAssign { lhs, rhs, out } => {
                if let Variable::Slice { item, .. } = out {
                    let offset = Variable::Named {
                        name: format!("{out}_offset"),
                        item: Item::Scalar(Elem::U32),
                        is_array: false,
                    };
                    let out = Variable::Named {
                        name: format!("(*{out}_ptr)"),
                        item: *item,
                        is_array: true,
                    };

                    index_assign(f, lhs, rhs, &out, Some(offset))
                } else {
                    index_assign(f, lhs, rhs, out, None)
                }
            }
            Instruction::If { cond, instructions } => {
                f.write_fmt(format_args!("if {cond} {{\n"))?;
                for i in instructions {
                    f.write_fmt(format_args!("{i}"))?;
                }
                f.write_str("}\n")
            }
            Instruction::IfElse {
                cond,
                instructions_if,
                instructions_else,
            } => {
                f.write_fmt(format_args!("if {cond} {{\n"))?;
                for i in instructions_if {
                    f.write_fmt(format_args!("{i}"))?;
                }
                f.write_str("} else {\n")?;
                for i in instructions_else {
                    f.write_fmt(format_args!("{i}"))?;
                }
                f.write_str("}\n")
            }
            Instruction::Return => f.write_str("return;\n"),
            Instruction::Break => f.write_str("break;\n"),
            Instruction::WorkgroupBarrier => f.write_str("workgroupBarrier();\n"),
            Instruction::StorageBarrier => f.write_str("storageBarrier();\n"),
            Instruction::Length { var, out } => match var {
                Variable::Slice { .. } => f.write_fmt(format_args!("{out} = {var}_length;\n")),
                _ => f.write_fmt(format_args!("{out} = arrayLength(&{var});\n")),
            },
            Instruction::Loop { instructions } => {
                f.write_fmt(format_args!("loop {{\n"))?;
                for i in instructions {
                    f.write_fmt(format_args!("{i}"))?;
                }
                f.write_str("}\n")
            }
            Instruction::BitwiseAnd { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = {lhs} & {rhs};\n"))
            }
            Instruction::BitwiseXor { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = {lhs} ^ {rhs};\n"))
            }
            Instruction::ShiftLeft { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = {lhs} << {rhs};\n"))
            }
            Instruction::ShiftRight { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = {lhs} >> {rhs};\n"))
            }
            Instruction::Floor { input, out } => {
                f.write_fmt(format_args!("{out} = floor({input});\n"))
            }
            Instruction::Ceil { input, out } => {
                f.write_fmt(format_args!("{out} = ceil({input});\n"))
            }
            Instruction::Subgroup(op) => f.write_fmt(format_args!("{op}")),
            Instruction::Bitcast { input, out } => {
                f.write_fmt(format_args!("{out} = bitcast<{}>({input});\n", out.elem()))
            }
            Instruction::AtomicLoad { input, out } => {
                f.write_fmt(format_args!("{out} = atomicLoad({input});\n"))
            }
            Instruction::AtomicStore { input, out } => {
                f.write_fmt(format_args!("atomicStore({out},{input});\n"))
            }
            Instruction::AtomicSwap { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicExchange({lhs}, {rhs});"))
            }
            Instruction::AtomicAdd { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicAdd({lhs}, {rhs});"))
            }
            Instruction::AtomicSub { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicSub({lhs}, {rhs});"))
            }
            Instruction::AtomicMax { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicMax({lhs}, {rhs});"))
            }
            Instruction::AtomicMin { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicMin({lhs}, {rhs});"))
            }
            Instruction::AtomicAnd { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicAnd({lhs}, {rhs});"))
            }
            Instruction::AtomicOr { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicOr({lhs}, {rhs});"))
            }
            Instruction::AtomicXor { lhs, rhs, out } => {
                f.write_fmt(format_args!("{out} = atomicXor({lhs}, {rhs});"))
            }
            Instruction::AtomicCompareExchangeWeak {
                lhs,
                cmp,
                value,
                out,
            } => f.write_fmt(format_args!(
                // For compatibility with cuda, only return old_value
                "{out} = atomicCompareExchangeWeak({lhs}, {cmp}, {value}).old_value;\n"
            )),
        }
    }
}

fn comparison(
    lhs: &Variable,
    rhs: &Variable,
    out: &Variable,
    op: &str,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match out.item() {
        Item::Vec4(_) => {
            let lhs0 = lhs.index(0);
            let lhs1 = lhs.index(1);
            let lhs2 = lhs.index(2);
            let lhs3 = lhs.index(3);
            let rhs0 = rhs.index(0);
            let rhs1 = rhs.index(1);
            let rhs2 = rhs.index(2);
            let rhs3 = rhs.index(3);

            f.write_fmt(format_args!(
                "
{out} = vec4({lhs0} {op} {rhs0}, {lhs1} {op} {rhs1}, {lhs2} {op} {rhs2}, {lhs3} {op} {rhs3});
"
            ))
        }
        Item::Vec3(_) => {
            let lhs0 = lhs.index(0);
            let lhs1 = lhs.index(1);
            let lhs2 = lhs.index(2);
            let rhs0 = rhs.index(0);
            let rhs1 = rhs.index(1);
            let rhs2 = rhs.index(2);

            f.write_fmt(format_args!(
                "
{out} = vec3({lhs0} {op} {rhs0}, {lhs1} {op} {rhs1}, {lhs2} {op} {rhs2});
"
            ))
        }
        Item::Vec2(_) => {
            let lhs0 = lhs.index(0);
            let lhs1 = lhs.index(1);
            let rhs0 = rhs.index(0);
            let rhs1 = rhs.index(1);

            f.write_fmt(format_args!(
                "
{out} = vec2({lhs0} {op} {rhs0}, {lhs1} {op} {rhs1});
"
            ))
        }
        Item::Scalar(_) => match rhs.item() {
            Item::Scalar(_) => f.write_fmt(format_args!("{out} = {lhs} {op} {rhs};\n")),
            _ => panic!("Can only compare a scalar when the output is a scalar"),
        },
    }
}

fn unroll<
    const N: usize,
    F: Fn(&mut core::fmt::Formatter<'_>, [IndexedVariable; N]) -> core::fmt::Result,
>(
    f: &mut core::fmt::Formatter<'_>,
    vectorization_factor: usize,
    variables: [&Variable; N],
    func: F,
) -> core::fmt::Result {
    for i in 0..vectorization_factor {
        let mut tmp = Vec::with_capacity(N);
        for var in variables.iter().take(N) {
            tmp.push(var.index(i));
        }
        let vars = tmp.try_into().unwrap();

        func(f, vars)?;
    }
    Ok(())
}

struct IndexOffset {
    var: Variable,
    offset: Option<Variable>,
    index: usize,
}
impl IndexOffset {
    fn new(var: &Variable, offset: &Option<Variable>, index: usize) -> Self {
        Self {
            var: var.clone(),
            offset: offset.clone(),
            index,
        }
    }
}

impl Display for IndexOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let var = self.var.index(self.index);

        match &self.offset {
            Some(offset) => {
                let offset = offset.index(self.index);
                f.write_fmt(format_args!("{var} + {offset}"))
            }
            None => f.write_fmt(format_args!("{var}")),
        }
    }
}

fn index(
    f: &mut std::fmt::Formatter<'_>,
    lhs: &Variable,
    rhs: &Variable,
    out: &Variable,
    offset: Option<Variable>,
) -> core::fmt::Result {
    if out.item().elem().is_atomic() {
        match offset {
            Some(offset) => f.write_fmt(format_args!("let {out} = &{lhs}[{rhs} + {offset}];\n")),
            None => f.write_fmt(format_args!("let {out} = &{lhs}[{rhs}];\n")),
        }
    } else if lhs.elem() != out.elem() {
        let item = out.item();
        match offset {
            Some(offset) => f.write_fmt(format_args!("{out} = {item}({lhs}[{rhs} + {offset}]);\n")),
            None => f.write_fmt(format_args!("{out} = {item}({lhs}[{rhs}]);\n")),
        }
    } else {
        match offset {
            Some(offset) => f.write_fmt(format_args!("{out} = {lhs}[{rhs} + {offset}];\n")),
            None => f.write_fmt(format_args!("{out} = {lhs}[{rhs}];\n")),
        }
    }
}

fn index_assign(
    f: &mut std::fmt::Formatter<'_>,
    lhs: &Variable,
    rhs: &Variable,
    out: &Variable,
    offset: Option<Variable>,
) -> core::fmt::Result {
    match lhs.item() {
        Item::Vec4(elem) => {
            let lhs0 = IndexOffset::new(lhs, &offset, 0);
            let lhs1 = IndexOffset::new(lhs, &offset, 1);
            let lhs2 = IndexOffset::new(lhs, &offset, 2);
            let lhs3 = IndexOffset::new(lhs, &offset, 3);

            let rhs0 = rhs.index(0);
            let rhs1 = rhs.index(1);
            let rhs2 = rhs.index(2);
            let rhs3 = rhs.index(3);

            f.write_fmt(format_args!("{out}[{lhs0}] = {elem}({rhs0});\n"))?;
            f.write_fmt(format_args!("{out}[{lhs1}] = {elem}({rhs1});\n"))?;
            f.write_fmt(format_args!("{out}[{lhs2}] = {elem}({rhs2});\n"))?;
            f.write_fmt(format_args!("{out}[{lhs3}] = {elem}({rhs3});\n"))
        }
        Item::Vec3(elem) => {
            let lhs0 = IndexOffset::new(lhs, &offset, 0);
            let lhs1 = IndexOffset::new(lhs, &offset, 1);
            let lhs2 = IndexOffset::new(lhs, &offset, 2);

            let rhs0 = rhs.index(0);
            let rhs1 = rhs.index(1);
            let rhs2 = rhs.index(2);

            f.write_fmt(format_args!("{out}[{lhs0}] = {elem}({rhs0});\n"))?;
            f.write_fmt(format_args!("{out}[{lhs1}] = {elem}({rhs1});\n"))?;
            f.write_fmt(format_args!("{out}[{lhs2}] = {elem}({rhs2});\n"))
        }
        Item::Vec2(elem) => {
            let lhs0 = IndexOffset::new(lhs, &offset, 0);
            let lhs1 = IndexOffset::new(lhs, &offset, 1);

            let rhs0 = rhs.index(0);
            let rhs1 = rhs.index(1);

            f.write_fmt(format_args!("{out}[{lhs0}] = {elem}({rhs0});\n"))?;
            f.write_fmt(format_args!("{out}[{lhs1}] = {elem}({rhs1});\n"))
        }
        Item::Scalar(_elem) => {
            let is_array = match out {
                Variable::GlobalInputArray(_, _)
                | Variable::GlobalOutputArray(_, _)
                | Variable::SharedMemory(_, _, _)
                | Variable::Slice { .. }
                | Variable::LocalArray(_, _, _, _) => true,
                Variable::Named { is_array, .. } => *is_array,
                _ => false,
            };

            if !is_array {
                let elem_out = out.elem();
                let casting_type = match rhs.item() {
                    Item::Vec4(_) => Item::Vec4(elem_out),
                    Item::Vec3(_) => Item::Vec3(elem_out),
                    Item::Vec2(_) => Item::Vec2(elem_out),
                    Item::Scalar(_) => Item::Scalar(elem_out),
                };
                f.write_fmt(format_args!("{out}[{lhs}] = {casting_type}({rhs});\n"))
            } else {
                let item_rhs = rhs.item();
                let item_out = out.item();
                let lhs = IndexOffset::new(lhs, &offset, 0);

                let vectorization_factor = item_out.vectorization_factor();
                if vectorization_factor > item_rhs.vectorization_factor() {
                    let casting_type = item_out.elem();
                    f.write_fmt(format_args!("{out}[{lhs}] = vec{vectorization_factor}("))?;
                    for i in 0..vectorization_factor {
                        let value = rhs.index(i);
                        f.write_fmt(format_args!("{casting_type}({value})"))?;

                        if i < vectorization_factor - 1 {
                            f.write_str(",")?;
                        }
                    }
                    f.write_str(");\n")
                } else {
                    f.write_fmt(format_args!("{out}[{lhs}] = {item_out}({rhs});\n"))
                }
            }
        }
    }
}
