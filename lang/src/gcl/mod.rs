use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::smt::SMT;
use rusmart_smt_stdlib::{Boolean, Cloak, Error, Integer, Map};

/// Value type representing different kinds of values in the language.
/// It can be an integer, boolean, or null.
#[smt_type]
pub enum Value {
    Integer(Integer),
    Boolean(Boolean),
    Null,
}

/// Expression type representing different kinds of executable expressions.
#[smt_type]
pub enum Expr {
    Value(Value), // A concrete value
    Undef,        // Undefined (variable not initialized)
    Error(Error), // An error condition (with an associated error token)
}

/// Variable type representing a variable identified by an integer ID.
#[smt_type]
pub struct Variable(Integer);

/// State type representing the current state of the program.
#[smt_type]
pub struct State {
    mem: Map<Variable, Expr>,
}

/// Assign a value to a variable in the state.
#[smt_impl(method = assign)]
pub fn assign(state: State, var: Variable, expr: Expr) -> State {
    State {
        mem: state.mem.put_unchecked(var, expr),
    }
}

/// Get the value of a variable from the state.
#[smt_impl(method = get)]
pub fn get(state: State, var: Variable) -> Expr {
    state.mem.get_unchecked(var)
}

/// Operators that can be applied to values.
#[smt_type]
pub enum Operator {
    LitBool(Boolean),
    LitInt(Integer),
    VarRef(Variable),
    // Binary arithmetic:
    Add(Cloak<Operator>, Cloak<Operator>),
    Sub(Cloak<Operator>, Cloak<Operator>),
    Mul(Cloak<Operator>, Cloak<Operator>),
    Div(Cloak<Operator>, Cloak<Operator>),
    // Comparison:
    Lt(Cloak<Operator>, Cloak<Operator>),
    Le(Cloak<Operator>, Cloak<Operator>),
    Gt(Cloak<Operator>, Cloak<Operator>),
    Ge(Cloak<Operator>, Cloak<Operator>),
    Eq(Cloak<Operator>, Cloak<Operator>),
    Ne(Cloak<Operator>, Cloak<Operator>),
    // Boolean connectives:
    And(Cloak<Operator>, Cloak<Operator>),
    Or(Cloak<Operator>, Cloak<Operator>),
    Xor(Cloak<Operator>, Cloak<Operator>),
    Not(Cloak<Operator>),
    Imply(Cloak<Operator>, Cloak<Operator>),
}

#[smt_impl(method = add)]
fn add_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => {
            // normal integer addition
            Expr::Value(Value::Integer(l.add(r)))
        }
        (Value::Null, _) | (_, Value::Null) => {
            // If either operand is Null (no value), propagate Null as result
            Expr::Value(Value::Null)
        }
        // Any type mismatch:
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = add)]
pub fn add(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        // Propagate errors or undef:
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        // Both are concrete values:
        (Expr::Value(v1), Expr::Value(v2)) => v1.add(v2),
    }
}

#[smt_impl(method = sub)]
fn sub_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => {
            // normal integer subtraction
            Expr::Value(Value::Integer(l.sub(r)))
        }
        (Value::Null, _) | (_, Value::Null) => {
            // If either operand is Null (no value), propagate Null as result
            Expr::Value(Value::Null)
        }
        // Any type mismatch:
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = sub)]
pub fn sub(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        // Propagate errors or undef:
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        // Both are concrete values:
        (Expr::Value(v1), Expr::Value(v2)) => v1.sub(v2),
    }
}

#[smt_impl(method = mul)]
fn mul_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => Expr::Value(Value::Integer(l.mul(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = mul)]
pub fn mul(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.mul(v2),
    }
}

#[smt_impl(method = div)]
fn div_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => {
            if *r.eq(Integer::from(0)) {
                // Division by zero is an error
                Expr::Error(Error::fresh())
            } else {
                // normal integer division
                Expr::Value(Value::Integer(l.div(r)))
            }
        }
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = div)]
pub fn div(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.div(v2),
    }
}

#[smt_impl(method = lt)]
fn lt_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => Expr::Value(Value::Boolean(l.lt(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = lt)]
pub fn lt(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.lt(v2),
    }
}

#[smt_impl(method = le)]
fn le_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => Expr::Value(Value::Boolean(l.le(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = le)]
pub fn le(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.le(v2),
    }
}

#[smt_impl(method = gt)]
fn gt_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => Expr::Value(Value::Boolean(l.gt(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = gt)]
pub fn gt(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.gt(v2),
    }
}

#[smt_impl(method = ge)]
fn ge_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => Expr::Value(Value::Boolean(l.ge(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = ge)]
pub fn ge(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.ge(v2),
    }
}

#[smt_impl(method = eq)]
fn eq_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => Expr::Value(Value::Boolean(l.eq(r))),
        (Value::Boolean(l), Value::Boolean(r)) => Expr::Value(Value::Boolean(l.eq(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        // Any type mismatch leads to an error
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = eq)]
pub fn eq(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.eq(v2),
    }
}

#[smt_impl(method = ne)]
fn ne_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => Expr::Value(Value::Boolean(l.ne(r))),
        (Value::Boolean(l), Value::Boolean(r)) => Expr::Value(Value::Boolean(l.ne(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = ne)]
pub fn ne(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.ne(v2),
    }
}

#[smt_impl(method = and)]
fn and_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Boolean(l), Value::Boolean(r)) => Expr::Value(Value::Boolean(l.and(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = and)]
pub fn and(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.and(v2),
    }
}

#[smt_impl(method = or)]
fn or_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Boolean(l), Value::Boolean(r)) => Expr::Value(Value::Boolean(l.or(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = or)]
pub fn or(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.or(v2),
    }
}

#[smt_impl(method = xor)]
fn xor_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Boolean(l), Value::Boolean(r)) => Expr::Value(Value::Boolean(l.xor(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = xor)]
pub fn xor(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.xor(v2),
    }
}

#[smt_impl(method = imply)]
fn imply_values(lhs: Value, rhs: Value) -> Expr {
    match (lhs, rhs) {
        (Value::Boolean(l), Value::Boolean(r)) => Expr::Value(Value::Boolean(l.implies(r))),
        (Value::Null, _) | (_, Value::Null) => Expr::Value(Value::Null),
        (_, _) => Expr::Error(Error::fresh()),
    }
}

#[smt_impl(method = imply)]
pub fn imply(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Error(e1), Expr::Error(e2)) => Expr::Error(e1.merge(e2)),
        (Expr::Error(e), _) | (_, Expr::Error(e)) => Expr::Error(e),
        (Expr::Undef, _) | (_, Expr::Undef) => Expr::Error(Error::fresh()),
        (Expr::Value(v1), Expr::Value(v2)) => v1.imply(v2),
    }
}

#[smt_impl]
pub fn evaluate_operator(state: State, op: Operator) -> Expr {
    match op {
        Operator::Add(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            add(lhs, rhs)
        }
        Operator::Sub(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            sub(lhs, rhs)
        }
        Operator::Mul(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            mul(lhs, rhs)
        }
        Operator::Div(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            div(lhs, rhs)
        }
        Operator::Lt(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            lt(lhs, rhs)
        }
        Operator::Le(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            le(lhs, rhs)
        }
        Operator::Gt(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            gt(lhs, rhs)
        }
        Operator::Ge(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            ge(lhs, rhs)
        }
        Operator::Eq(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            eq(lhs, rhs)
        }
        Operator::Ne(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            ne(lhs, rhs)
        }
        Operator::And(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            and(lhs, rhs)
        }
        Operator::Or(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            or(lhs, rhs)
        }
        Operator::Xor(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            xor(lhs, rhs)
        }
        Operator::LitBool(v) => Expr::Value(Value::Boolean(v)),
        Operator::LitInt(v) => Expr::Value(Value::Integer(v)),
        Operator::VarRef(v) => get(state, v),
        Operator::Not(op) => {
            let op = evaluate_operator(state, op.reveal());
            match op {
                Expr::Value(Value::Boolean(b)) => Expr::Value(Value::Boolean(b.not())),
                _ => Expr::Error(Error::fresh()),
            }
        }
        Operator::Imply(lhs, rhs) => {
            let lhs = evaluate_operator(state, lhs.reveal());
            let rhs = evaluate_operator(state, rhs.reveal());
            imply(lhs, rhs)
        }
    }
}

/// Statements that can be executed in the language.
#[smt_type]
pub enum Statement {
    SSkip,
    SAssign(Variable, Operator),
    SIf(Operator, Cloak<Statement>, Cloak<Statement>),
    SWhile(Operator, Cloak<Statement>),
    SSeq(Cloak<Statement>, Cloak<Statement>),
}

#[smt_impl]
pub fn evaluate_statement(state: State, stmt: Statement) -> (State, Expr) {
    match stmt {
        Statement::SSkip => (state, Expr::Value(Value::Null)),
        Statement::SAssign(var, op) => {
            // Evaluate the RHS expression
            let op_eval = evaluate_operator(state, op);
            match op_eval {
                Expr::Error(e) => (state, Expr::Error(e)),
                Expr::Undef => (state, Expr::Error(Error::fresh())),
                Expr::Value(_) => {
                    // Assign the resulting expr_val to the variable
                    let new_state = State {
                        mem: state.mem.put_unchecked(var, op_eval),
                    };
                    (new_state, op_eval)
                }
            }
        }
        Statement::SIf(cond_op, then_stmt, else_stmt) => {
            // Evaluate condition
            let cond_val = evaluate_operator(state, cond_op);
            match cond_val {
                Expr::Error(e) => (state, Expr::Error(e)), // condition error
                Expr::Undef => (state, Expr::Error(Error::fresh())), // undefined condition
                Expr::Value(Value::Boolean(b)) => {
                    if *b {
                        // condition true: execute then branch
                        evaluate_statement(state, then_stmt.reveal())
                    } else {
                        // condition false: execute else branch
                        evaluate_statement(state, else_stmt.reveal())
                    }
                }
                _ => {
                    // cond_val is not a Boolean
                    (state, Expr::Error(Error::fresh()))
                }
            }
        }
        Statement::SWhile(cond_op, body_stmt) => {
            // Evaluate the condition:
            let cond_val = evaluate_operator(state, cond_op);
            match cond_val {
                Expr::Error(e) => (state, Expr::Error(e)),
                Expr::Undef => (state, Expr::Error(Error::fresh())),
                Expr::Value(Value::Boolean(b)) => {
                    if *b {
                        // Execute body, then loop again
                        let (s_body, res) = evaluate_statement(state, body_stmt.reveal());
                        if let Expr::Error(e) = res {
                            // If body resulted in error, propagate it
                            (s_body, Expr::Error(e))
                        } else {
                            // otherwise, evaluate loop again with updated state
                            evaluate_statement(s_body, Statement::SWhile(cond_op, body_stmt))
                        }
                    } else {
                        // condition false -> loop ends, do nothing more
                        (state, Expr::Value(Value::Null))
                    }
                }
                _ => {
                    // condition evaluated to a non-bool
                    (state, Expr::Error(Error::fresh()))
                }
            }
        }
        Statement::SSeq(s1, s2) => {
            // Execute first, then second
            let (state2, res1) = evaluate_statement(state, s1.reveal());
            if let Expr::Error(e) = res1 {
                // if first part errors, stop
                (state2, Expr::Error(e))
            } else {
                // otherwise execute second
                evaluate_statement(state2, s2.reveal())
            }
        }
    }
}

/// A program is a sequence of statements.
#[smt_type]
pub struct Program {
    pub statements: Statement,
}

#[smt_impl]
pub fn evaluate_program(prog: Program) -> (State, Expr) {
    let initial_state = State { mem: Map::new() };
    let statement = prog.statements;
    evaluate_statement(initial_state, statement)
}

#[smt_spec(impls = [evaluate_operator])]
pub fn spec_evaluate_operator(_s: State, _op: Operator) -> Expr {
    unimplemented!()
}

#[smt_spec(impls = [evaluate_statement])]
pub fn spec_evaluate_statement(_s: State, _st: Statement) -> (State, Expr) {
    unimplemented!()
}

#[smt_spec(impls = [evaluate_program])]
pub fn spec_evaluate_program(_prog: Program) -> (State, Expr) {
    unimplemented!()
}

#[smt_axiom]
pub fn axiom1(p1: Program, p2: Program) -> Boolean {
    p1.eq(p2)
        .implies(spec_evaluate_program(p1).eq(spec_evaluate_program(p2)))
}

// determinism axiom
#[smt_axiom]
pub fn axiom2(s: State, st: Statement, out1: (State, Expr), out2: (State, Expr)) -> Boolean {
    spec_evaluate_statement(s, st)
        .eq(out1)
        .and(spec_evaluate_statement(s, st).eq(out2))
        .implies(out1.eq(out2))
}

#[smt_axiom]
pub fn axiom3(p1: Program, s1: Statement) -> Boolean {
    let Program { statements } = p1;
    let empty_state = State { mem: Map::new() };
    statements
        .eq(s1)
        .implies(spec_evaluate_program(p1).eq(spec_evaluate_statement(empty_state, s1)))
}

/// Programs must produce identical results when evaluated multiple times with the same initial state
#[smt_axiom]
pub fn axiom4(p: Program) -> Boolean {
    spec_evaluate_program(p).eq(spec_evaluate_program(p))
}

/// Equivalence Transitivity
#[smt_axiom]
pub fn axiom5(p1: Program, p2: Program, p3: Program) -> Boolean {
    spec_evaluate_program(p1)
        .eq(spec_evaluate_program(p2))
        .and(spec_evaluate_program(p2).eq(spec_evaluate_program(p3)))
        .implies(spec_evaluate_program(p1).eq(spec_evaluate_program(p3)))
}
