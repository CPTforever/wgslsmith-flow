use std::fmt::{Display, Write};

use indenter::indented;

use crate::{ExprNode, Postfix};

#[derive(Debug, PartialEq, Eq)]
pub enum AssignmentLhs {
    Underscore,
    Simple(String, Vec<Postfix>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    LetDecl(String, ExprNode),
    VarDecl(String, ExprNode),
    Assignment(AssignmentLhs, ExprNode),
    Compound(Vec<Statement>),
    If(ExprNode, Vec<Statement>),
    Return(Option<ExprNode>),
}

impl Statement {
    /// Extracts the inner statements from a `Statement::CompoundStatement`.
    ///
    /// This will panic if `self` is not a `Statement::CompoundStatement`.
    pub fn into_compount_statement(self) -> Vec<Statement> {
        match self {
            Statement::Compound(stmts) => stmts,
            _ => unreachable!(),
        }
    }
}

impl Display for AssignmentLhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignmentLhs::Underscore => f.write_char('_'),
            AssignmentLhs::Simple(name, postfixes) => {
                f.write_str(name)?;

                for postfix in postfixes {
                    match postfix {
                        Postfix::ArrayIndex(index) => write!(f, "[{}]", index)?,
                        Postfix::Member(field) => write!(f, ".{}", field)?,
                    }
                }

                Ok(())
            }
        }
    }
}

impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::LetDecl(name, value) => write!(f, "let {} = {};", name, value),
            Statement::VarDecl(name, value) => write!(f, "var {} = {};", name, value),
            Statement::Assignment(lhs, rhs) => write!(f, "{} = {};", lhs, rhs),
            Statement::Compound(stmts) => {
                writeln!(f, "{{")?;

                for stmt in stmts {
                    writeln!(indented(f), "{}", stmt)?;
                }

                write!(f, "}}")
            }
            Statement::If(cond, stmts) => {
                writeln!(f, "if ({}) {{", cond)?;

                for stmt in stmts {
                    writeln!(indented(f), "{}", stmt)?;
                }

                write!(f, "}}")
            }
            Statement::Return(value) => {
                write!(f, "return")?;

                if let Some(value) = value {
                    write!(f, " {}", value)?;
                }

                write!(f, ";")
            }
        }
    }
}
