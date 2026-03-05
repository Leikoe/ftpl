use ftpl::*;

fn main() {
    println!("--- Example 06: Zero-Copy vs. Performance-Free Transformations ---");
    let valuation = Valuation::new();

    // 1. Initial Layout: Row-major [4, 8]
    let s4_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l_orig = Expression::Linearize(s4_8.clone());

    // 2. The transformation: .T.reshape(4, 8)
    // Transpose to (8, 4) then reshape back to (4, 8)
    let s8_4 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
    ]);
    let transpose = Expression::Permute(s8_4.clone(), vec![1, 0]);
    let reshape = Expression::Reshape(s4_8.clone(), s8_4.clone());
    
    // Final Layout = Linearize(4, 8) o Transpose(8->4) o Reshape(4->8)
    let l_final = Expression::Composition(
        Box::new(reshape),
        Box::new(Expression::Composition(Box::new(transpose), Box::new(l_orig.clone())))
    );

    // 3. Symbolic Comparison
    println!("Checking if the transformation is an identity (No-Op):");
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
    let lowered_orig = l_orig.lower(&valuation, inputs.clone());
    let lowered_final = l_final.lower(&valuation, inputs);

    println!("  Original Address: {:?}", lowered_orig[0]);
    println!("  Final Address:    {:?}", lowered_final[0].clone().simplify());

    println!("\nCONCLUSION:");
    if lowered_orig[0] == lowered_final[0].clone().simplify() {
        println!("  - This is a NO-OP (Identity). It is computationally FREE.");
    } else {
        println!("  - This is NOT an identity. The mapping has changed.");
        println!("  - Zero-Copy? YES. (Stored purely in metadata).");
        println!("  - Performance-Free? NO. (Index math now involves expensive Div/Mod).");
        println!("\n  To regain high-performance (contiguous strides), a compiler");
        println!("  would need to insert a physical copy to reorganize the data.");
    }
}
