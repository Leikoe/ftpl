use ftpl::*;

fn main() {
    println!("=== Example 19: Hardware Instruction Fit (mma.m16n8k16) ===\n");
    let valuation = Valuation::new();

    // 1. DEFINE THE HARDWARE PRIMITIVE (mma.m16n8k16)
    let s_a_frag = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(16), Some("Row".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(16), Some("Col".to_string())),
    ]);
    let s_a_hw = Space::new(vec![
        Factor::new(Kind::Execution, Extent::Constant(32), Some("Thread".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(8), Some("Reg".to_string())),
    ]);
    let mma_a_instr = Expression::Reshape(s_a_frag.clone(), s_a_hw.clone());

    let s_b_frag = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(16), Some("Row".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("Col".to_string())),
    ]);
    let s_b_hw = Space::new(vec![
        Factor::new(Kind::Execution, Extent::Constant(32), Some("Thread".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(4), Some("Reg".to_string())),
    ]);
    let mma_b_instr = Expression::Reshape(s_b_frag.clone(), s_b_hw.clone());

    println!("Defined Hardware Primitives:");
    println!("  MMA_A: 16x16 Logical -> (32 Threads x 8 Regs)");
    println!("  MMA_B: 16x8  Logical -> (32 Threads x 4 Regs)");

    // 2. DEFINE THE PROGRAM LAYOUT (128x128 GEMM)
    let s_a_tiles = Space::new(vec![
        Factor::new(Kind::Tile, Extent::Constant(8), Some("TileR".to_string())),
        Factor::new(Kind::Tile, Extent::Constant(8), Some("TileC".to_string())),
    ]);
    let l_a_program = Expression::Product(
        Box::new(mma_a_instr.clone()),
        Box::new(Expression::Identity(s_a_tiles.clone()))
    );

    let s_b_tiles = Space::new(vec![
        Factor::new(Kind::Tile, Extent::Constant(8), Some("TileR".to_string())),
        Factor::new(Kind::Tile, Extent::Constant(16), Some("TileC".to_string())),
    ]);
    let l_b_program = Expression::Product(
        Box::new(mma_b_instr.clone()),
        Box::new(Expression::Identity(s_b_tiles.clone()))
    );

    // 3. THE FORMAL FIT CHECK
    println!("\nPerforming Formal Fit Checks (Factorization):");
    if let Some(remain_a) = l_a_program.clone().left_div(mma_a_instr) {
        println!("  [✓] A-Operand: SUCCESS. Instruction fits.");
        println!("      Remaining Loop Space: {:?}", remain_a.source().volume_extent().try_eval(&valuation.variables).unwrap());
    }
    if let Some(remain_b) = l_b_program.clone().left_div(mma_b_instr) {
        println!("  [✓] B-Operand: SUCCESS. Instruction fits.");
        println!("      Remaining Loop Space: {:?}", remain_b.source().volume_extent().try_eval(&valuation.variables).unwrap());
    }

    // 4. CODEGEN
    println!("\nLowered Hardware Math for Operands:");
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1), ScalarExpr::Input(2), ScalarExpr::Input(3)];
    let addr_a = l_a_program.lower(&valuation, inputs.clone());
    let addr_b = l_b_program.lower(&valuation, inputs);

    println!("  A Address Components:");
    for (i, expr) in addr_a.0.iter().enumerate() {
        println!("    Target Dim {}: {}", i, viz::cuda::to_cuda(&expr.clone().simplify(), &["r", "c", "tr", "tc"]));
    }
    println!("  B Address Components:");
    for (i, expr) in addr_b.0.iter().enumerate() {
        println!("    Target Dim {}: {}", i, viz::cuda::to_cuda(&expr.clone().simplify(), &["r", "c", "tr", "tc"]));
    }
}
