use ftpl::*;

fn main() {
    println!("--- Example 06: Zero-Copy vs. Performance-Free Transformations ---");
    let valuation = Valuation::new();

    // 1. Initial Layout: Row-major [4, 8]
    let l_orig = Layout::row_major((4, 8));

    // 2. The transformation: .T.reshape(4, 8)
    let l_final = l_orig.clone()
        .transpose()
        .reshape((4, 8));

    // 3. Symbolic Comparison using equivalent_to()
    println!("Checking if the transformation is an identity (No-Op):");
    println!("  Is L_orig equivalent to L_final? {}", l_orig.equivalent_to(&l_final));

    // 4. Manual address comparison
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
    let addr_orig = l_orig.lower(&valuation, inputs.clone());
    let addr_final = l_final.lower(&valuation, inputs);

    println!("\nLowered Math Comparison:");
    println!("  Original Address: {}", viz::cuda::to_cuda(&addr_orig.0[0].clone().simplify(), &["h", "w"]));
    println!("  Final Address:    {}", viz::cuda::to_cuda(&addr_final.0[0].clone().simplify(), &["h", "w"]));

    println!("\nCONCLUSION:");
    println!("  Although stored in metadata (Zero-Copy), the complex index math");
    println!("  proves that the hardware would prefer a physical copy to restore");
    println!("  simple unit-stride contiguity.");
}
