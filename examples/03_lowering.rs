use ftpl::*;

fn main() {
    println!("--- Example 03: Symbolic Lowering ---");

    // 1. Define a 2D Linearized Space [H=4, W=8]
    let s = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let lin = Expression::Linearize(s);

    // 2. Symbolic Lowering: Convert the layout to a flat scalar expression
    let valuation = Valuation::new();
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]; // (i0, i1)
    let lowered = lin.lower(&valuation, inputs);

    // 3. Final Device-Side Expression
    println!("Device-Side Math: {:?}", lowered[0]);
    // It should represent (i0 * 8) + i1
    
    // 4. Simulate Device Execution
    let result = lowered[0].eval(&[1, 2]); // i0=1, i1=2
    println!("Device Simulated Execution result (1, 2) -> {}", result);
}
