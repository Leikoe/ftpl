use ftpl::*;

fn main() {
    println!("// === SM80 FP64 Atom: The 'Linear Layouts' Paper Style ===\n");
    let valuation = Valuation::new();

    // 1. In the paper, spaces are built from bits (F2).
    // Let's define the 6 bits of an 8x8 logical space.
    let bit = Extent::Constant(2);
    let s_logical_bits = Space::new(vec![
        Factor::new(Kind::Logical, bit.clone(), Some("m0".to_string())),
        Factor::new(Kind::Logical, bit.clone(), Some("m1".to_string())),
        Factor::new(Kind::Logical, bit.clone(), Some("m2".to_string())),
        Factor::new(Kind::Logical, bit.clone(), Some("n0".to_string())),
        Factor::new(Kind::Logical, bit.clone(), Some("n1".to_string())),
        Factor::new(Kind::Logical, bit.clone(), Some("n2".to_string())),
    ]);

    // 2. Define the Hardware bits (Registers and Threads).
    let s_hw_bits = Space::new(vec![
        Factor::new(Kind::Storage,   bit.clone(), Some("r0".to_string())), // RegID
        Factor::new(Kind::Execution, bit.clone(), Some("t0".to_string())), // ThreadID bits
        Factor::new(Kind::Execution, bit.clone(), Some("t1".to_string())),
        Factor::new(Kind::Execution, bit.clone(), Some("t2".to_string())),
        Factor::new(Kind::Execution, bit.clone(), Some("t3".to_string())),
        Factor::new(Kind::Execution, bit.clone(), Some("t4".to_string())),
    ]);

    // 3. The Paper Style: Define the mapping as a permutation of bits.
    // Based on SM80 FP64 C-operand logic:
    // m = [t0, t1, r0], n = [t2, t3, t4]
    //
    // The mapping is:
    // HW Bit 0 (r0) -> Logical Bit 2 (m2)
    // HW Bit 1 (t0) -> Logical Bit 0 (m0)
    // HW Bit 2 (t1) -> Logical Bit 1 (m1)
    // HW Bit 3 (t2) -> Logical Bit 3 (n0)
    // HW Bit 4 (t3) -> Logical Bit 4 (n1)
    // HW Bit 5 (t4) -> Logical Bit 5 (n2)
    
    let bit_permutation = vec![2, 0, 1, 3, 4, 5]; // target_bit = perm[hw_bit]
    let atom_matrix = Expression::Permute(s_hw_bits.clone(), bit_permutation);

    println!("Constructed Linear Layout via Bit-Permutation Matrix.");
    println!("Matrix Columns: [r0, t0, t1, t2, t3, t4]");
    println!("Matrix Rows:    [m0, m1, m2, n0, n1, n2]");

    // 4. Verification: Apply to a specific Thread/Register
    // Let's check Thread 9 (binary 01001) and Register 1 (binary 1).
    // HW bits = [r0=1, t0=1, t1=0, t2=0, t3=1, t4=0]
    let hw_coord = vec![1, 1, 0, 0, 1, 0];
    let logical_bits = atom_matrix.apply(&valuation, &hw_coord).unwrap();
    
    println!("\nApplying Layout to (Thread 9, Reg 1):");
    println!("  Logical Bits [m0, m1, m2, n0, n1, n2] = {:?}", logical_bits);
    // Expected m = [1, 0, 1] = 5, n = [0, 1, 0] = 2.
    
    // 5. Final Hardware Math (The Bit-Logic)
    let inputs = vec![
        ScalarExpr::Input(0), ScalarExpr::Input(1), ScalarExpr::Input(2),
        ScalarExpr::Input(3), ScalarExpr::Input(4), ScalarExpr::Input(5)
    ];
    let (addr, _) = atom_matrix.lower(&valuation, inputs);
    
    println!("\nLowered Bit-Level Hardware Math:");
    for (i, expr) in addr.iter().enumerate() {
        println!("  Logical Bit {}: {}", i, viz::cuda::to_cuda(&expr.clone().simplify(), &["r0", "t0", "t1", "t2", "t3", "t4"]));
    }

    println!("\nCONCLUSION:");
    println!("  In the paper, every complex hardware atom is just one of these ");
    println!("  bit-matrices. Our library's 'Permute' and 'BinaryShadow' ");
    println!("  are the direct implementations of this mathematical style.");
}
