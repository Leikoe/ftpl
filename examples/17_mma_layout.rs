use ftpl::*;

fn main() {
    println!("=== Example 17: Tensor Core MMA Layouts ===\n");
    let valuation = Valuation::new();

    // 1. Hardware Space: 32 threads, each with 8 registers.
    let s_mma_hw = Space::new(vec![
        Factor::new(Kind::Execution, Extent::Constant(32), Some("ThreadID".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(8), Some("RegID".to_string())),
    ]);

    // 2. Logical Space: 16x16 Matrix Fragment (M, N)
    let s_logical = Space::from((16, 16));

    // 3. Construct the MMA Mapping
    let mma_layout = Layout::new(Expression::Reshape(s_logical, s_mma_hw));

    println!("Constructed 16x16 MMA Layout.");

    // 4. Verification: The "Primitive Fit" Check
    // Global layout is a tiled version: (4x4 tiles of 16x16)
    let global_layout = mma_layout.clone().product(Layout::identity((4, 4)));

    println!("\nChecking Hardware Fit:");
    if let Some(_) = global_layout.expr.clone().left_div(mma_layout.expr) {
        println!("  [✓] SUCCESS: 16x16 MMA Instruction fits the 64x64 Global Layout!");
    } else {
        println!("  [!] FAILURE: Incompatible tiling.");
    }

    // 5. CUDA Codegen
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1), ScalarExpr::Input(2), ScalarExpr::Input(3)];
    let addr = global_layout.lower(&valuation, inputs);
    println!("\nLowered Address Components:");
    for (i, expr) in addr.0.iter().enumerate() {
        println!("  Target Dim {}: {}", i, viz::cuda::to_cuda(&expr.clone().simplify(), &["r", "c", "tr", "tc"]));
    }
}
