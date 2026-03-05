use ftpl::*;

fn main() {
    println!("--- Example 05: Structural Judgments ---");

    // 1. Injective: One-to-one mapping
    let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
    let id = Expression::Identity(s.clone());
    println!("Is Identity injective? {:?}", id.is_injective());

    // 2. Aliasing (Broadcasting): Multiple logical points map to the same point
    let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(1), None)]);
    let broadcast = Expression::Broadcast(s.clone(), s1.clone());
    
    // Judgment requires a valuation for volume-based analysis
    let valuation = Valuation::new();
    println!("Is Broadcast aliasing? {:?}", broadcast.is_aliasing(&valuation));
    
    // 3. Surjective: Covers the entire target space
    println!("Is Broadcast surjective? {:?}", broadcast.is_surjective());
}
