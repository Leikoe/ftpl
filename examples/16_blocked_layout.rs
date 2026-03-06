use ftpl::*;

fn main() {
    println!("=== Example 16: Blocked Layouts (The Triton Pattern) ===\n");
    let valuation = Valuation::new();

    // 1. Logical Tensor: [Height=16, Width=16]
    let s_logical = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(16), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(16), Some("W".to_string())),
    ]);

    // 2. Hardware Hierarchy: Warp x Thread x Register
    let s_hw = Space::new(vec![
        Factor::new(Kind::Execution, Extent::Constant(2), Some("WarpID".to_string())),
        Factor::new(Kind::Execution, Extent::Constant(8), Some("ThreadID".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(4), Some("RegID".to_string())),
    ]);

    // 3. Construct the Blocked Layout using high-level API
    let layout = Layout::new(Expression::Reshape(s_logical, s_hw.clone()));

    println!("Constructed Blocked Layout.");
    println!("Source: [16, 16] -> Target: [Warp:2, Thread:8, Reg:4]");

    // 4. Verification: Structural Properties
    println!("\nStructural Analysis:");
    println!("  Is Injective?  {:?}", layout.is_injective());
    println!("  Is Surjective? {:?}", layout.is_surjective());
    
    // 5. Codegen: What does the thread see?
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]; // (h, w)
    let addr = layout.lower(&valuation, inputs);
    println!("\nLowered Hardware Indexing:");
    println!("  Target (Warp, Thread, Reg) = {:?}", addr.0[0].clone().simplify());
}
