use ftpl::*;

fn main() {
    let valuation = Valuation::new();
    let s8_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l1 = Expression::Linearize(s8_8.clone());
    let s_col_target = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("H".to_string())),
    ]);
    let l_col = Expression::Composition(
        Box::new(Expression::Linearize(s_col_target.clone())),
        Box::new(Expression::Permute(s8_8.clone(), vec![1, 0]))
    );
    let shuffle = l1.shuffle_to(&l_col).unwrap();
    println!("Shuffle = {:#?}", shuffle);
    let inv = l1.inverse().unwrap();
    let mid = inv.apply(&valuation, &[1]);
    println!("Inv out = {:?}", mid);
    let p = Expression::Permute(s8_8.clone(), vec![1, 0]);
    let mid2 = p.apply(&valuation, &mid.clone().unwrap());
    println!("Permute out = {:?}", mid2);
    let lin = Expression::Linearize(s_col_target.clone());
    let out = lin.apply(&valuation, &mid2.unwrap());
    println!("Lin out = {:?}", out);
}
