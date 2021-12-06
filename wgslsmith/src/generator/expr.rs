use rand::prelude::{IteratorRandom, SliceRandom, StdRng};
use rand::Rng;

use crate::ast::{BinOp, Expr, ExprNode, Lit, UnOp};
use crate::types::{DataType, ScalarType, TypeConstraints};

use super::scope::Scope;

pub struct ExprGenerator<'a> {
    rng: &'a mut StdRng,
    scope: &'a mut Scope,
    depth: u32,
}

#[derive(Clone, Copy, Debug)]
enum ExprType {
    Lit,
    TypeCons,
    Var,
    UnOp,
    BinOp,
}

impl<'a> ExprGenerator<'a> {
    pub fn new(rng: &'a mut StdRng, scope: &'a mut Scope) -> ExprGenerator<'a> {
        ExprGenerator {
            rng,
            scope,
            depth: 0,
        }
    }

    pub fn gen_expr(&mut self, constraints: &TypeConstraints) -> ExprNode {
        log::info!(
            "generating expr with {:?}, depth={}",
            constraints,
            self.depth
        );

        let mut allowed = vec![];

        if constraints.intersects(TypeConstraints::Scalar()) {
            allowed.push(ExprType::Lit);
        }

        if constraints.intersects(TypeConstraints::Vec()) {
            allowed.push(ExprType::TypeCons);
        }

        if self.depth < 5 {
            allowed.push(ExprType::UnOp);

            if constraints.intersects(&TypeConstraints::Scalar().union(TypeConstraints::VecInt())) {
                allowed.push(ExprType::BinOp);
            }

            if self.scope.intersects(constraints) {
                allowed.push(ExprType::Var);
            }
        }

        log::info!("allowed constructions: {:?}", allowed);

        match *allowed.choose(&mut self.rng).unwrap() {
            ExprType::Lit => {
                let (lit, t) = self.gen_lit(constraints);
                ExprNode {
                    data_type: t,
                    expr: Expr::Lit(lit),
                }
            }
            ExprType::TypeCons => {
                log::info!("generating type_cons with {:?}", constraints);

                let data_type = constraints
                    .intersection(TypeConstraints::Vec())
                    .select(&mut self.rng);

                log::info!("generating type cons with t={}", data_type);

                let mut args = vec![];

                let (n, t) = match data_type {
                    DataType::Scalar(t) => (1, t),
                    DataType::Vector(n, t) => (n, t),
                };

                let constraints = DataType::Scalar(t).into();
                for _ in 0..n {
                    args.push(self.gen_expr(&constraints))
                }

                ExprNode {
                    data_type,
                    expr: Expr::TypeCons(data_type, args),
                }
            }
            ExprType::UnOp => {
                self.depth += 1;

                let op = self.gen_un_op(constraints);
                let constraints = match op {
                    UnOp::Neg => constraints
                        .intersection(&TypeConstraints::I32().union(TypeConstraints::VecI32())),
                    UnOp::Not => constraints
                        .intersection(&TypeConstraints::Bool().union(TypeConstraints::VecBool())),
                    UnOp::BitNot => constraints
                        .intersection(&TypeConstraints::Int().union(TypeConstraints::VecInt())),
                };

                let expr = self.gen_expr(&constraints);

                self.depth -= 1;

                ExprNode {
                    data_type: expr.data_type,
                    expr: Expr::UnOp(op, Box::new(expr)),
                }
            }
            ExprType::BinOp => {
                self.depth += 1;

                let op = self.gen_bin_op(constraints);
                let lconstraints = match op {
                    BinOp::Plus
                    | BinOp::Minus
                    | BinOp::Times
                    | BinOp::Divide
                    | BinOp::Mod
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXOr
                    | BinOp::LShift
                    | BinOp::RShift => constraints
                        .intersection(&TypeConstraints::Int().union(TypeConstraints::VecInt())),
                    BinOp::LogAnd | BinOp::LogOr => {
                        constraints.intersection(TypeConstraints::Bool())
                    }
                };

                let l = self.gen_expr(&lconstraints);
                let rconstraints = match op {
                    // For shifts, right operand must be u32
                    BinOp::LShift | BinOp::RShift => match l.data_type {
                        DataType::Scalar(_) => TypeConstraints::U32().clone(),
                        DataType::Vector(n, _) => DataType::Vector(n, ScalarType::U32).into(),
                    },
                    // For everything else right operand must be same type as left
                    _ => l.data_type.into(),
                };

                let r = self.gen_expr(&rconstraints);

                self.depth -= 1;

                ExprNode {
                    data_type: l.data_type,
                    expr: Expr::BinOp(op, Box::new(l), Box::new(r)),
                }
            }
            ExprType::Var => {
                log::info!(
                    "generating var with {:?}, scope={:?}",
                    constraints,
                    self.scope
                );

                let (name, &data_type) = self
                    .scope
                    .iter()
                    .filter(|(_, t)| constraints.intersects(&(*t).into()))
                    .choose(&mut self.rng)
                    .unwrap();

                ExprNode {
                    data_type,
                    expr: Expr::Var(name.to_owned()),
                }
            }
        }
    }

    fn gen_lit(&mut self, constraints: &TypeConstraints) -> (Lit, DataType) {
        log::info!("generating lit with {:?}", constraints);

        // Select a random concrete type from the constraints
        let t = constraints
            .intersection(TypeConstraints::Scalar())
            .select(&mut self.rng);

        log::info!("generating lit with t={}", t);

        let lit = match t {
            DataType::Scalar(t) => match t {
                ScalarType::Bool => Lit::Bool(self.rng.gen()),
                ScalarType::I32 => Lit::Int(self.rng.gen()),
                ScalarType::U32 => Lit::UInt(self.rng.gen()),
            },
            _ => unreachable!(),
        };

        (lit, t)
    }

    fn gen_un_op(&mut self, constraints: &TypeConstraints) -> UnOp {
        log::info!("generating un_op with {:?}", constraints);

        let mut allowed = vec![];

        if constraints.intersects(&TypeConstraints::I32().union(TypeConstraints::VecI32())) {
            allowed.push(UnOp::Neg);
        }

        if constraints.intersects(&TypeConstraints::Bool().union(TypeConstraints::VecBool())) {
            allowed.push(UnOp::Not);
        }

        if constraints.intersects(&TypeConstraints::Int().union(TypeConstraints::VecInt())) {
            allowed.push(UnOp::BitNot)
        }

        log::info!("allowed constructions: {:?}", allowed);

        *allowed.choose(&mut self.rng).unwrap()
    }

    fn gen_bin_op(&mut self, constraints: &TypeConstraints) -> BinOp {
        log::info!("generating bin_op with {:?}", constraints);

        let mut allowed = vec![];

        if constraints.intersects(&TypeConstraints::Int().union(TypeConstraints::VecInt())) {
            allowed.extend_from_slice(&[
                BinOp::Plus,
                BinOp::Minus,
                BinOp::Times,
                BinOp::Divide,
                BinOp::Mod,
                BinOp::BitAnd,
                BinOp::BitOr,
                BinOp::BitXOr,
                BinOp::LShift,
                BinOp::RShift,
            ]);
        }

        if constraints.intersects(TypeConstraints::Bool()) {
            // TODO: Non short-circuiting logical & and | are currently broken in naga
            // https://github.com/gfx-rs/naga/issues/1574
            allowed.extend_from_slice(&[BinOp::LogAnd, BinOp::LogOr]);
        }

        log::info!("allowed constructions: {:?}", allowed);

        *allowed.choose(&mut self.rng).unwrap()
    }
}
