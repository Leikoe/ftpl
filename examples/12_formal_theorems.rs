use ftpl::*;

fn main() {
    println!("=== Example 12: Advanced Formal Theorems ===\n");
    let valuation = Valuation::new();

    // 1. Formal Equivalence (L1 ≡ L2)
    println!("--- 1. Automated Equivalence Checking ---");
    let s8_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l1 = Layout::row_major(s8_8.clone());

    // T(T(A)) == A
    let l2 = l1.clone().transpose().transpose();

    println!("Is L1 equivalent to L2? {}", l1.equivalent_to(&l2));


    // 2. The Shuffle Theorem (Data Movement)
    println!("\n--- 2. The Shuffle Theorem ---");
    let l_col = l1.clone().transpose();
    
    if let Some(shuffle) = l1.shuffle_to(&l_col) {
        println!("Successfully generated Shuffle Layout.");
        let result = shuffle.apply(&valuation, &[1]).unwrap();
        println!("Shuffle Address Mapping for L1_offset[1] -> L_col_offset: {:?}", result);
    } else {
        println!("Failed to generate shuffle.");
    }


    // 3. Bank-Conflict Detection
    println!("\n--- 3. Bank Conflict Analysis ---");
    let hw_target = Space::new(vec![
        Factor::new(Kind::Execution, Extent::Constant(32), Some("ThreadIdx".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(32), Some("Offset".to_string())),
    ]);
    let l_bad = Layout::row_major(hw_target);
    let conflicts = l_bad.bank_conflict_strides(&valuation);
    
    for (idx, stride) in conflicts {
        println!("Execution Dimension {} has Storage Stride {}", idx, stride);
        if stride % 32 == 0 && stride != 0 {
            println!("  [!] SEVERE BANK CONFLICT DETECTED.");
        }
    }
}
