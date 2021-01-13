use crate::{
    env::Env,
    result::{TyError, TyErrorKind, TyResult},
    synth::Synth,
};

use liquid_rust_mir::Rvalue;
use liquid_rust_ty::{BaseTy, BinOp, Predicate, Ty, UnOp, Variable};

impl<'env> Synth<'env, ()> for Rvalue {
    type Ty = Ty;
    type Envs = &'env Env;

    fn synth(&self, env: Self::Envs) -> TyResult<(), Self::Ty> {
        match self {
            Rvalue::Use(operand) => operand.synth(env),
            Rvalue::UnApp(un_op, op) => {
                let (param_ty, ret_ty) = match un_op {
                    // the `-` operator receives an integer and returns an integer of the same
                    // type.
                    UnOp::Neg(sign, size) => (BaseTy::Int(*sign, *size), BaseTy::Int(*sign, *size)),
                    // the `!` operator receives a booolean and returns a boolean.
                    UnOp::Not => (BaseTy::Bool, BaseTy::Bool),
                };

                // Synthetize the type of the operand
                let op_ty1 = op.synth(env)?;
                // The type of the operand must have the type that the operator receives as base
                // type.
                if !op_ty1.has_base(param_ty) {
                    return Err(TyError {
                        kind: TyErrorKind::BaseMismatch {
                            expected: param_ty,
                            found: op_ty1.clone(),
                        },
                        span: (),
                    });
                }

                // Resolve the operand into a predicate. This is possible because operands are
                // literals or locals.
                let op = Box::new(env.resolve_operand(op));

                // Return the `{ b : B | b == (un_op op) }` type.
                Ok(Ty::Refined(
                    ret_ty,
                    Predicate::Var(Variable::Bound).eq(ret_ty, Predicate::UnaryOp(*un_op, op)),
                ))
            }
            Rvalue::BinApp(bin_op, op1, op2) => {
                let (op_ty, ret_ty) = match bin_op {
                    // Arithmetic operators receive two integers of the same type and return an
                    // integer of the same type.
                    BinOp::Add(sign, size)
                    | BinOp::Sub(sign, size)
                    | BinOp::Mul(sign, size)
                    | BinOp::Div(sign, size)
                    | BinOp::Rem(sign, size) => {
                        (BaseTy::Int(*sign, *size), BaseTy::Int(*sign, *size))
                    }
                    // Rust's MIR does not have boolean binary operators. They are here just to be
                    // reused in predicates.
                    BinOp::And | BinOp::Or => unreachable!(),
                    // Equality operators receive two operands of the same type and return a
                    // boolean.
                    BinOp::Eq(ty) | BinOp::Neq(ty) => (*ty, BaseTy::Bool),
                    // Comparison operators receive two integers of the same type and return a
                    // boolean.
                    BinOp::Lt(sign, size)
                    | BinOp::Gt(sign, size)
                    | BinOp::Lte(sign, size)
                    | BinOp::Gte(sign, size) => (BaseTy::Int(*sign, *size), BaseTy::Bool),
                };
                // Synthetize the types of the operands.
                let op_ty1 = op1.synth(env)?;
                let op_ty2 = op2.synth(env)?;

                // The type of the operands should be the same.
                //
                // FIXME: this is not the case for the offset and shift operators.
                if !op_ty1.shape_eq(&op_ty2) {
                    return Err(TyError {
                        kind: TyErrorKind::ShapeMismatch {
                            expected: op_ty1.clone(),
                            found: op_ty2.clone(),
                        },
                        span: (),
                    });
                }
                // The type of the operands must have the type that the operator receives as base
                // type.
                if !op_ty1.has_base(op_ty) {
                    return Err(TyError {
                        kind: TyErrorKind::BaseMismatch {
                            expected: op_ty,
                            found: op_ty1.clone(),
                        },
                        span: (),
                    });
                }

                // Resolve the operands into predicates. This is possible because operands are
                // literals or locals.
                let op1 = Box::new(env.resolve_operand(op1));
                let op2 = Box::new(env.resolve_operand(op2));

                // Return the `{ b : B | b == (op1 bin_op op2) }` type.
                Ok(Ty::Refined(
                    ret_ty,
                    Predicate::Var(Variable::Bound)
                        .eq(ret_ty, Predicate::BinaryOp(*bin_op, op1, op2)),
                ))
            }
        }
    }
}