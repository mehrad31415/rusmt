use z3::{Config, Context, Solver, ast::Int};
use std::ops::Add;

fn main() {
    // Step 1: Set up Z3 context
    let cfg = Config::new();
    let ctx = Context::new(&cfg);

    // Step 2: Create a solver
    let solver = Solver::new(&ctx);

    // Step 3: Create two integer variables
    let x = Int::new_const(&ctx, "x");
    let y = Int::new_const(&ctx, "y");

    // Step 4: Add constraints: x + y == 10, and x > 0
    solver.assert(&x.add(&[&y])._eq(&Int::from_i64(&ctx, 10)));
    solver.assert(&x.gt(&Int::from_i64(&ctx, 0)));

    // Step 5: Check satisfiability
    match solver.check() {
        z3::SatResult::Sat => {
            println!("SAT");
            let model = solver.get_model().unwrap();
            let x_val = model.eval(&x, true).unwrap();
            let y_val = model.eval(&y, true).unwrap();
            println!("x = {}, y = {}", x_val, y_val);
        }
        z3::SatResult::Unsat => println!("UNSAT"),
        z3::SatResult::Unknown => println!("UNKNOWN"),
    }
}