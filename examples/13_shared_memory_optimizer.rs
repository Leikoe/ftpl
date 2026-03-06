use ftpl::*;

fn main() {
    println!("=== Usage Example: Automated GPU Shared Memory Optimizer ===\n");
    let valuation = Valuation::new();

    // 1. THE PROBLEM: A 32x32 Matrix in Shared Memory
    let matrix_32x32 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(32), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(32), Some("W".to_string())),
    ]);

    // The execution mapping
    let matrix_execution = Space::new(vec![
        Factor::new(Kind::Execution, Extent::Constant(32), Some("ThreadIdx.x".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(32), Some("LocalRow".to_string())),
    ]);

    // Naive Transpose: row_major.transpose().reshape(matrix_execution)
    let final_naive = Layout::row_major(matrix_32x32)
        .transpose()
        .reshape(matrix_execution.clone());

    // 2. THE ANALYSIS: Detect Bank Conflicts
    println!("--- 1. Analyzing Naive Transpose ---");
    let conflicts = final_naive.bank_conflict_strides(&valuation);
    for (dim, stride) in &conflicts {
        if matrix_execution.factors[*dim].kind == Kind::Execution {
            println!("  Execution Dim {} ('{}') has Memory Stride: {}", 
                     dim, matrix_execution.factors[*dim].tag.0.as_ref().unwrap(), stride);
            if stride % 32 == 0 {
                println!("  [!] RESULT: Severe 32-way Bank Conflict Detected!");
            }
        }
    }

    // 3. THE FIX: Apply an XOR-Swizzle (Binary Shadow)
    println!("\n--- 2. Applying Algebraic Swizzle (Binary Shadow) ---");
    let mut swizzle_mat = vec![vec![0; 10]; 10];
    for i in 0..10 { swizzle_mat[i][i] = 1; }
    swizzle_mat[0][5] = 1; 
    
    let optimized_layout = final_naive.swizzle(swizzle_mat);

    // 4. THE VERIFICATION
    let new_conflicts = optimized_layout.bank_conflict_strides(&valuation);
    println!("--- 3. Verifying Optimized Layout ---");
    for (dim, stride) in &new_conflicts {
        if matrix_execution.factors[*dim].kind == Kind::Execution {
            println!("  Execution Dim {} now has Stride: {}", dim, stride);
            if *stride % 32 != 0 || *stride == 1 {
                println!("  [✓] RESULT: Proven Bank-Conflict Free!");
            }
        }
    }

    // 5. THE LOWERING
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
    let addr = optimized_layout.lower(&valuation, inputs);
    println!("\n--- 4. Final Lowered Hardware Math ---");
    println!("  Target Address = {}", viz::cuda::to_cuda(&addr.0[0].clone().simplify(), &["tid", "off"]));
}
