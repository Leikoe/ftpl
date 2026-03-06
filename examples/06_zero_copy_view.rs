use ftpl::*;

fn main() {
    println!("--- Example 06: FTPL Zero-Copy View Fusion ---");
    let valuation = Valuation::new();

    // 1. Initial [4, 8] Row-Major Layout
    let l_orig = Layout::row_major((4, 8));

    // 2. The transformation: .T.reshape(4, 8)
    let l_final = l_orig.transpose().reshape((4, 8));

    // 3. Symbolic Lowering (The "Free" Proof)
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
    let addr = l_final.lower(&valuation, inputs);

    println!("[FTPL Analysis]");
    println!("  The transformation is composed of 3 stages, but the compiler");
    println!("  fuses them into a single device-side scalar expression.");

    println!("\n  Final Lowered Expression: {:?}", addr.0[0].clone().simplify());
    
    println!("\nCONCLUSION:");
    println!("  In FTPL, this is a ZERO-COPY operation.");
    println!("  The hardware simply uses a different stride pattern to access the same memory.");
    println!("  No data movement is required because the layout is structural.");
}
