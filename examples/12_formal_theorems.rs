use ftpl::*;

fn main() {
    println!("=== Example 12: Advanced Formal Theorems ===\n");
    let valuation = Valuation::new();

    // ---------------------------------------------------------
    // THEOREM 1: Formal Equivalence (L1 ≡ L2)
    // ---------------------------------------------------------
    println!("--- 1. Automated Equivalence Checking ---");
    // Layout 1: [8, 8] Row-Major
    let s8_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l1 = Expression::Linearize(s8_8.clone());

    // Layout 2: Transpose -> Transpose -> Row-Major
    // Since T(T(A)) == A, this should be mathematically equivalent to L1.
    let transpose_view = Expression::Permute(s8_8.clone(), vec![1, 0]);
    let l2 = Expression::Composition(
        Box::new(transpose_view.clone()),
        Box::new(Expression::Composition(
            Box::new(transpose_view),
            Box::new(l1.clone()),
        )),
    );

    println!("Is L1 equivalent to L2? {}", l1.equivalent_to(&l2));

    // ---------------------------------------------------------
    // THEOREM 2: The Shuffle Theorem (Data Movement)
    // ---------------------------------------------------------
    println!("\n--- 2. The Shuffle Theorem ---");
    // We want to generate a layout that physically moves data from
    // a Row-Major layout (L1) to a Column-Major layout (L_col).
    // Both must have the SAME logical source space: [H=8, W=8].
    // Column major is: Linearize([W=8, H=8]) o Permute([H=8, W=8] -> [W=8, H=8])
    let s_col_target = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("H".to_string())),
    ]);
    let l_col = Expression::Composition(
        Box::new(Expression::Permute(s8_8.clone(), vec![1, 0])),
        Box::new(Expression::Linearize(s_col_target.clone())),
    );

    // To move data from L1 to L_col, we generate: L_shuffle = L_col o L1^-1
    // This represents the read/write address mapping required for the copy.
    if let Some(shuffle) = l1.shuffle_to(&l_col) {
        println!("Successfully generated Shuffle Layout.");

        // Let's test a coordinate: Offset 1 in L1 belongs to Logical (0, 1).
        // In L_col (Column Major), Logical (0, 1) lives at offset 8.
        // Therefore, Shuffle(1) should return 8!
        let target_offset = shuffle.apply(&valuation, &[1]).expect("apply failed");
        println!(
            "Shuffle Address Mapping for L1_offset[1] -> L_col_offset: {:?}",
            target_offset
        );
    } else {
        println!("Failed to generate shuffle (Layout is not bijective).");
    }

    // ---------------------------------------------------------
    // THEOREM 3: Bank-Conflict Detection
    // ---------------------------------------------------------
    println!("\n--- 3. Bank Conflict Analysis ---");
    // Imagine mapping a 32x32 matrix to a GPU warp (32 threads).
    // Threads iterate over Width.
    let hw_target = Space::new(vec![
        Factor::new(
            Kind::Execution,
            Extent::Constant(32),
            Some("ThreadIdx".to_string()),
        ),
        Factor::new(
            Kind::Storage,
            Extent::Constant(32),
            Some("Offset".to_string()),
        ),
    ]);

    // Bad Layout: Column-Major distribution.
    // Thread 0 gets col 0 (offsets 0, 32, 64...) -> ALL hit Bank 0!
    // Thread 1 gets col 1 (offsets 1, 33, 65...) -> ALL hit Bank 1!
    // If threads read simultaneously, stride = 32. 32 % 32 banks == 0 -> MASSIVE CONFLICT.

    let l_bad = Expression::Linearize(hw_target);
    let conflicts = l_bad.bank_conflict_strides(&valuation);

    for (idx, stride) in conflicts {
        println!("Execution Dimension {} has Storage Stride {}", idx, stride);
        let num_banks = 32;
        if stride % num_banks == 0 && stride != 0 {
            println!(
                "  [!] SEVERE BANK CONFLICT DETECTED: Stride {} is a multiple of {} banks.",
                stride, num_banks
            );
        } else if stride > 1 {
            println!(
                "  [!] Potential uncoalesced access detected (Stride {}).",
                stride
            );
        } else {
            println!("  [✓] Fully coalesced, conflict-free access.");
        }
    }
}
