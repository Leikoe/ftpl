use ftpl::*;

fn main() {
    println!("--- Example 02: Layout Views (Transpose & Reshape) ---");

    // 1. Define initial row-major layout
    let l_orig = Layout::row_major((4, 8));
    
    // 2. Transpose: (4, 8) -> (8, 4)
    let l_transposed = l_orig.clone().transpose();
    println!("Transpose Target Space: {:?}", l_transposed.target().volume_extent());

    // 3. Reshape: Flatten (4, 8) into (32)
    let l_reshaped = l_orig.reshape(32);
    println!("Reshape Target Space: {:?}", l_reshaped.target().volume_extent());

    // 4. Coordinate Mapping & CUDA Codegen
    let valuation = Valuation::new();
    
    // The source space of l_reshaped is now [32]
    let inputs = vec![ScalarExpr::Input(0)]; // i0 = flat index
    let addr = l_reshaped.lower(&valuation, inputs);
    
    println!("\nLowered CUDA Math for Reshaped Layout:");
    println!("  Address = {}", viz::cuda::to_cuda(&addr.0[0].clone().simplify(), &["idx"]));

    // 5. Verification
    let input_coord = vec![10]; 
    let output_coord = l_reshaped.apply(&valuation, &input_coord).unwrap();
    println!("\nVerification: Coordinate (10) in [32] maps to offset {:?} in memory", output_coord);
}
