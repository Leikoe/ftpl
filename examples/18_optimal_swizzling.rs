use ftpl::*;

fn main() {
    println!("=== Example 18: Optimal Swizzling (The Appendix Algorithm) ===\n");
    let valuation = Valuation::new();

    // 1. Setup the Conflict Scenario:
    let l_naive = Layout::row_major((32, 32)).transpose();

    println!("--- 1. Naive Access Pattern ---");
    let conflicts = l_naive.bank_conflict_strides(&valuation);
    for (_, stride) in &conflicts {
        if stride % 32 == 0 {
            println!("  [!] Detected 32-way Bank Conflict (Stride={})", stride);
        }
    }

    // 2. The Appendix Optimization:
    let mut swizzle_matrix = vec![vec![0; 10]; 10];
    for i in 0..10 { swizzle_matrix[i][i] = 1; }
    swizzle_matrix[0][5] = 1;

    let l_optimized = l_naive.swizzle(swizzle_matrix);

    println!("\n--- 2. Optimized Access Pattern (After Appendix Swizzle) ---");
    let new_conflicts = l_optimized.bank_conflict_strides(&valuation);
    for (_, stride) in &new_conflicts {
        if *stride == 1 {
            println!("  [✓] Conflict Resolved: Unit Stride Proven!");
        }
    }

    // 3. Final Algebraic Verification
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
    let addr = l_optimized.lower(&valuation, inputs);
    println!("\nFinal Theorem Check:");
    println!("  CUDA Math = {}", viz::cuda::to_cuda(&addr.0[0].clone().simplify(), &["r", "c"]));
}
