use std::collections::HashSet;

use ast::{
    AssignmentLhs, Expr, ExprNode, ForLoopInit, ForLoopUpdate, LhsExpr, LhsExprNode, Module,
    Postfix, Statement,
};

pub fn remove_accessed_vars(vars: &mut HashSet<String>, module: &Module) {
    for decl in &module.functions {
        for stmt in &decl.body {
            visit_stmt(vars, stmt);
        }
    }
}

fn visit_stmt(vars: &mut HashSet<String>, stmt: &Statement) {
    match stmt {
        Statement::LetDecl(decl) => visit_expr(vars, &decl.initializer),
        Statement::VarDecl(decl) => {
            if let Some(init) = &decl.initializer {
                visit_expr(vars, init);
            }
        }
        Statement::Assignment(stmt) => {
            match &stmt.lhs {
                AssignmentLhs::Phony => {}
                AssignmentLhs::Expr(expr) => visit_lhs_expr(vars, expr),
            }

            visit_expr(vars, &stmt.rhs);
        }
        Statement::Compound(stmts) => {
            for stmt in stmts {
                visit_stmt(vars, stmt);
            }
        }
        Statement::If(stmt) => {
            visit_expr(vars, &stmt.condition);

            for stmt in &stmt.body {
                visit_stmt(vars, stmt);
            }

            let mut else_ = stmt.else_.as_deref();
            while let Some(e) = else_ {
                match e {
                    ast::Else::If(stmt) => {
                        visit_expr(vars, &stmt.condition);

                        for stmt in &stmt.body {
                            visit_stmt(vars, stmt);
                        }

                        else_ = stmt.else_.as_deref();
                    }
                    ast::Else::Else(body) => {
                        for stmt in body {
                            visit_stmt(vars, stmt);
                        }

                        else_ = None;
                    }
                }
            }
        }
        Statement::Return(stmt) => {
            if let Some(e) = &stmt.value {
                visit_expr(vars, e);
            }
        }
        Statement::Loop(stmt) => {
            for stmt in &stmt.body {
                visit_stmt(vars, stmt);
            }
        }
        Statement::Break => {}
        Statement::Switch(stmt) => {
            visit_expr(vars, &stmt.selector);

            for case in &stmt.cases {
                for stmt in &case.body {
                    visit_stmt(vars, stmt);
                }
            }

            for stmt in &stmt.default {
                visit_stmt(vars, stmt);
            }
        }
        Statement::ForLoop(stmt) => {
            if let Some(init) = &stmt.header.init {
                match init {
                    ForLoopInit::VarDecl(stmt) => {
                        if let Some(init) = &stmt.initializer {
                            visit_expr(vars, init);
                        }
                    }
                }
            }

            if let Some(condition) = &stmt.header.condition {
                visit_expr(vars, condition);
            }

            if let Some(update) = &stmt.header.update {
                match update {
                    ForLoopUpdate::Assignment(stmt) => {
                        match &stmt.lhs {
                            AssignmentLhs::Phony => {}
                            AssignmentLhs::Expr(expr) => visit_lhs_expr(vars, expr),
                        }

                        visit_expr(vars, &stmt.rhs);
                    }
                }
            }

            for stmt in &stmt.body {
                visit_stmt(vars, stmt);
            }
        }
    }
}

fn visit_lhs_expr(vars: &mut HashSet<String>, node: &LhsExprNode) {
    match &node.expr {
        LhsExpr::Ident(ident) => {
            vars.remove(ident);
        }
        LhsExpr::Postfix(expr, postfix) => {
            visit_lhs_expr(vars, expr);
            visit_postfix(vars, postfix);
        }
    }
}

fn visit_expr(vars: &mut HashSet<String>, node: &ExprNode) {
    match &node.expr {
        Expr::Lit(_) => {}
        Expr::TypeCons(_, args) => {
            for arg in args {
                visit_expr(vars, arg);
            }
        }
        Expr::Var(ident) => {
            vars.remove(ident.as_str());
        }
        Expr::Postfix(expr, postfix) => {
            visit_expr(vars, expr);
            visit_postfix(vars, postfix);
        }
        Expr::UnOp(_, expr) => visit_expr(vars, expr),
        Expr::BinOp(_, left, right) => {
            visit_expr(vars, left);
            visit_expr(vars, right);
        }
        Expr::FnCall(_, args) => {
            for arg in args {
                visit_expr(vars, arg);
            }
        }
    }
}

fn visit_postfix(vars: &mut HashSet<String>, postfix: &Postfix) {
    match postfix {
        Postfix::ArrayIndex(index) => visit_expr(vars, index),
        Postfix::Member(_) => {}
    }
}
