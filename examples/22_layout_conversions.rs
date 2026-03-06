use ftpl::*;

fn main() {
    println!("// === Example 22: Mathematical Layout Conversions ===\n");
    let valuation = Valuation::new();

    // 1. Define Layout 1: Row-Major [8, 8]
    let l1 = Layout::row_major((8, 8));

    // 2. Define Layout 2: Column-Major [8, 8]
    let l2 = l1.clone().transpose();

    println!("Layout 1 (Source): Row-Major");
    println!("Layout 2 (Target): Column-Major");

    // 3. Calculate the Conversion (Relative Layout)
    if let Some(conversion) = l1.shuffle_to(&l2) {
        println!("\nSuccessfully calculated Mathematical Conversion.");
        
        // 4. Generate the "Shuffle Index"
        let inputs = vec![ScalarExpr::Input(0)]; // input is the source offset
        let addr = conversion.lower(&valuation, inputs);
        
        println!("\nGenerated Shuffle Formula (SrcOffset -> TgtOffset):");
        println!("  TargetOffset = {}", viz::cuda::to_cuda(&addr.0[0].clone().simplify(), &["src_off"]));

        // 5. Verification
        let result = conversion.apply(&valuation, &[1]).unwrap();
        println!("\nVerification: Mapping SrcOffset 1 -> TgtOffset {:?}", result);
    } else {
        println!("Conversion failed.");
    }

    println!("\nCONCLUSION:");
    println!("  Conversion between layouts is not an ad-hoc 'buggy' implementation.");
    println!("  It is a mathematically derived composition: Target o Source^-1.");
}
