use ftpl::*;

fn main() {
    println!("--- Example 06: FTPL Zero-Copy View Fusion ---");
    let valuation = Valuation::new();

    // 1. Initial [4, 8] Row-Major Layout
    let s4_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l_orig = Expression::Linearize(s4_8.clone());

    // 2. The transformation: .T.reshape(4, 8)
    // In FTPL, these are composed as virtual views.
    let s8_4 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
    ]);
    let transpose = Expression::Permute(s8_4.clone(), vec![1, 0]);
    let reshape = Expression::Reshape(s4_8.clone(), s8_4.clone());
    
    // L_final = L_orig o Transpose o Reshape
    let l_final = Expression::Composition(
        Box::new(reshape),
        Box::new(Expression::Composition(Box::new(transpose), Box::new(l_orig)))
    );

    // 3. Symbolic Lowering (The "Free" Proof)
    // We pass symbolic variables through the entire chain.
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
    let lowered = l_final.lower(&valuation, inputs);

    println!("[FTPL Analysis]");
    println!("  The transformation is composed of 3 stages, but the compiler");
    println!("  fuses them into a single device-side scalar expression.");
    println!("\n  Final Lowered Expression: {:?}", lowered.0[0].clone().simplify());
    
    println!("\nCONCLUSION:");
    println!("  Is it 'Free'? Yes and No.");
    println!("  - YES: FTPL can represent it purely as metadata (Zero-Copy).");
    println!("  - NO:  The generated expression contains expensive Div and Mod operations!");
    println!("         `((i1 + i0 * 8) / 4) % 8 + ((i1 + i0 * 8) % 4) * 8`");
    println!("         This proves the transformation broke contiguity.");
    println!("         To get back to cheap affine strides, the compiler must insert a physical copy.");
}
