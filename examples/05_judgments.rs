use ftpl::*;

fn main() {
    println!("--- Example 05: Structural Judgments ---");

    // 1. Injective Check: Identity is injective
    let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
    let id = Layout::identity(s.clone());
    println!("Is Identity injective? {:?}", id.is_injective());

    // 2. Aliasing (Broadcasting): Repeat is aliasing
    // Mapping [10] -> [1]
    let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(1), None)]);
    let broadcast = Layout::new(Expression::Broadcast(s.clone(), s1));
    
    let valuation = Valuation::new();
    println!("Is Broadcast aliasing? {:?}", broadcast.is_aliasing(&valuation));
    
    // 3. Surjective: Repeat covers its smaller target
    println!("Is Broadcast surjective? {:?}", broadcast.is_surjective());
}
