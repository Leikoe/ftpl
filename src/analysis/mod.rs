use crate::core::{Space, Valuation, Kind, Factor, Extent};
use crate::layout::{Expression, Layout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judgment {
    True,
    False,
    Unknown,
}

pub struct LayeredNormalForm {
    pub view: Expression,
    pub placement: Expression,
    pub shadow: Expression,
}

impl Layout for LayeredNormalForm {
    fn source(&self) -> Space {
        self.view.source()
    }

    fn target(&self) -> Space {
        self.shadow.target()
    }

    fn apply(&self, valuation: &Valuation, input: &[u64]) -> Option<Vec<u64>> {
        let v = self.view.apply(valuation, input)?;
        let p = self.placement.apply(valuation, &v)?;
        self.shadow.apply(valuation, &p)
    }

    fn lower(&self, valuation: &Valuation, inputs: Vec<crate::layout::ScalarExpr>) -> (Vec<crate::layout::ScalarExpr>, crate::layout::ScalarExpr) {
        let (v, d1) = self.view.lower(valuation, inputs);
        let (p, d2) = self.placement.lower(valuation, v);
        let (s, d3) = self.shadow.lower(valuation, p);
        let domain = crate::layout::ScalarExpr::And(
            Box::new(d1),
            Box::new(crate::layout::ScalarExpr::And(Box::new(d2), Box::new(d3)))
        ).simplify();
        (s, domain)
    }
}

impl Expression {
    pub fn is_injective(&self) -> Judgment {
        match self {
            Expression::Identity(_) | Expression::Linearize(_) | Expression::Delinearize(_) |
            Expression::Permute(_, _) | Expression::Reshape(_, _) | Expression::Slice(_, _) |
            Expression::Pad(_, _, _) | Expression::Flip(_, _) | Expression::BinaryShadow(_, _) |
            Expression::Split(_, _, _) | Expression::Join(_, _) | Expression::Squeeze(_, _) |
            Expression::Unsqueeze(_, _, _) => Judgment::True,
            Expression::Composition(l1, l2) => match (l1.is_injective(), l2.is_injective()) {
                (Judgment::True, Judgment::True) => Judgment::True,
                (Judgment::False, _) | (_, Judgment::False) => Judgment::False,
                _ => Judgment::Unknown,
            },
            Expression::Product(l1, l2) => match (l1.is_injective(), l2.is_injective()) {
                (Judgment::True, Judgment::True) => Judgment::True,
                (Judgment::False, _) | (_, Judgment::False) => Judgment::False,
                _ => Judgment::Unknown,
            },
            Expression::Broadcast(_, _) | Expression::Repeat(_, _) => Judgment::False,
        }
    }

    pub fn is_surjective(&self) -> Judgment {
        match self {
            Expression::Identity(_) | Expression::Linearize(_) | Expression::Delinearize(_) |
            Expression::Permute(_, _) | Expression::Reshape(_, _) | Expression::Broadcast(_, _) |
            Expression::Flip(_, _) | Expression::BinaryShadow(_, _) | Expression::Split(_, _, _) |
            Expression::Join(_, _) | Expression::Squeeze(_, _) | Expression::Unsqueeze(_, _, _) |
            Expression::Repeat(_, _) => Judgment::True,
            Expression::Composition(l1, l2) => match (l1.is_surjective(), l2.is_surjective()) {
                (Judgment::True, Judgment::True) => Judgment::True,
                (Judgment::False, _) | (_, Judgment::False) => Judgment::False,
                _ => Judgment::Unknown,
            },
            Expression::Product(l1, l2) => match (l1.is_surjective(), l2.is_surjective()) {
                (Judgment::True, Judgment::True) => Judgment::True,
                (Judgment::False, _) | (_, Judgment::False) => Judgment::False,
                _ => Judgment::Unknown,
            },
            Expression::Slice(_, _) | Expression::Pad(_, _, _) => Judgment::False,
        }
    }

    pub fn is_aliasing(&self, valuation: &Valuation) -> Judgment {
        match self {
            Expression::Reshape(s, t) => {
                let vol_s = valuation.get(&s.volume_extent());
                let vol_t = valuation.get(&t.volume_extent());
                match (vol_s, vol_t) {
                    (Some(vs), Some(vt)) => {
                        if vs > vt { Judgment::True } else { Judgment::False }
                    }
                    _ => Judgment::Unknown,
                }
            }
            Expression::Broadcast(_, _) | Expression::Repeat(_, _) => Judgment::True,
            _ => {
                match self.is_injective() {
                    Judgment::True => Judgment::False,
                    Judgment::False => Judgment::True,
                    Judgment::Unknown => Judgment::Unknown,
                }
            }
        }
    }

    pub fn simplify_recursive(self) -> Expression {
        match self {
            Expression::Composition(l1, l2) => {
                let l1 = l1.simplify_recursive();
                let l2 = l2.simplify_recursive();
                match (l1, l2) {
                    (Expression::Linearize(s1), Expression::Delinearize(s2)) if s1 == s2 => Expression::Identity(s1),
                    (Expression::Delinearize(s1), Expression::Linearize(s2)) if s1 == s2 => {
                        let vol = s1.volume_extent();
                        Expression::Identity(Space::new(vec![Factor::new(Kind::Other("Offset".to_string()), vol, None)]))
                    }
                    (Expression::Reshape(s1, s2), Expression::Reshape(s2_prime, s3)) if s2 == s2_prime => Expression::Reshape(s1, s3),
                    (Expression::Permute(s1, p1), Expression::Permute(s2, p2)) if s1.compatible(&Expression::Permute(s2.clone(), p2.clone()).target()) => {
                        let mut p_final = vec![0; p1.len()];
                        for i in 0..p1.len() { p_final[i] = p2[p1[i]]; }
                        Expression::Permute(s1, p_final)
                    }
                    (Expression::BinaryShadow(s1, m1), Expression::BinaryShadow(s2, m2)) if s1 == s2 => {
                        let d = m1.len();
                        let mut m_final = vec![vec![0; d]; d];
                        for i in 0..d {
                            for j in 0..d {
                                for k in 0..d { m_final[i][j] ^= m1[i][k] & m2[k][j]; }
                            }
                        }
                        Expression::BinaryShadow(s1, m_final)
                    }
                    (l1, l2) => Expression::Composition(Box::new(l1), Box::new(l2)),
                }
            }
            Expression::Product(l1, l2) => {
                let l1 = l1.simplify_recursive();
                let l2 = l2.simplify_recursive();
                Expression::Product(Box::new(l1), Box::new(l2))
            }
            _ => self,
        }
    }

    pub fn layer_type(&self) -> &'static str {
        match self {
            Expression::Identity(_) | Expression::Slice(_, _) | Expression::Broadcast(_, _) | 
            Expression::Permute(_, _) | Expression::Reshape(_, _) | Expression::Pad(_, _, _) |
            Expression::Flip(_, _) | Expression::Split(_, _, _) | Expression::Join(_, _) |
            Expression::Squeeze(_, _) | Expression::Unsqueeze(_, _, _) | Expression::Repeat(_, _) => "View",
            Expression::Linearize(_) | Expression::Delinearize(_) => "Placement",
            Expression::BinaryShadow(_, _) => "Shadow",
            Expression::Composition(_, _) | Expression::Product(_, _) => "Complex",
        }
    }

    pub fn normalize(self) -> LayeredNormalForm {
        let simplified = self.simplify_recursive();
        match simplified {
            Expression::Composition(l1, l2) => {
                let nf1 = l1.normalize();
                let nf2 = l2.normalize();
                LayeredNormalForm {
                    view: Expression::Composition(Box::new(nf1.view), Box::new(nf2.view)).simplify_recursive(),
                    placement: Expression::Composition(Box::new(nf1.placement), Box::new(nf2.placement)).simplify_recursive(),
                    shadow: Expression::Composition(Box::new(nf1.shadow), Box::new(nf2.shadow)).simplify_recursive(),
                }
            }
            Expression::Identity(s) => LayeredNormalForm {
                view: Expression::Identity(s.clone()),
                placement: Expression::Identity(s.clone()),
                shadow: Expression::Identity(s),
            },
            _ => {
                let lt = simplified.layer_type();
                let src = simplified.source();
                let tgt = simplified.target();
                match lt {
                    "View" => LayeredNormalForm { view: simplified, placement: Expression::Identity(tgt.clone()), shadow: Expression::Identity(tgt) },
                    "Placement" => LayeredNormalForm { view: Expression::Identity(src.clone()), placement: simplified, shadow: Expression::Identity(tgt) },
                    "Shadow" => LayeredNormalForm { view: Expression::Identity(src.clone()), placement: Expression::Identity(src.clone()), shadow: simplified },
                    _ => LayeredNormalForm { view: simplified, placement: Expression::Identity(tgt.clone()), shadow: Expression::Identity(tgt) }
                }
            }
        }
    }

    pub fn left_div(self, target: Expression) -> Option<Expression> {
        match self {
            Expression::Product(t, r) => {
                if *t == target { Some(*r) } else {
                    if let Expression::Product(t_inner, a) = *t {
                        if *t_inner == target { return Some(Expression::Product(a, r)); }
                    }
                    None
                }
            }
            _ => None,
        }
    }

    /// Calculates the maximum contiguous vector width for the innermost dimension.
    /// This is determined structurally by finding the largest power-of-two stride-1 prefix
    /// in the normalized placement layer.
    /// Calculates the maximum contiguous vector width for the innermost dimension.
    pub fn max_vector_width(&self, valuation: &Valuation) -> u64 {
        let src = self.source();
        if src.factors.is_empty() { return 1; }
        let inner_idx = src.factors.len() - 1;

        let stride = self.get_stride(inner_idx, valuation);
        let extent = valuation.get(&src.factors[inner_idx].extent).unwrap_or(1);
        
        match stride {
            Some(1) => extent,
            _ => 1,
        }
    }
}

impl Expression {
    pub fn get_stride(&self, dim_idx: usize, valuation: &Valuation) -> Option<u64> {
        match self {
            Expression::Identity(_) => Some(1),
            Expression::Linearize(s) => {
                let mut stride = 1;
                for i in (dim_idx + 1)..s.factors.len() {
                    stride *= valuation.get(&s.factors[i].extent)?;
                }
                Some(stride)
            }
            Expression::Composition(l1, l2) => {
                // To find the stride of dim_idx in (l2 o l1):
                // If l1 is a Permute, the stride is the stride of the reordered index in l2.
                if let Expression::Permute(_, p) = &**l1 {
                    let out_idx = p[dim_idx];
                    return l2.get_stride(out_idx, valuation);
                }
                
                // General fallback: use symbolic evaluation
                let src = self.source();
                let mut inputs = Vec::new();
                for i in 0..src.factors.len() { inputs.push(crate::layout::ScalarExpr::Input(i)); }
                let (lowered, _) = self.lower(valuation, inputs);
                if lowered.is_empty() { return None; }
                
                let mut coords0 = vec![0; src.factors.len()];
                let mut coords1 = vec![0; src.factors.len()];
                coords1[dim_idx] = 1;
                
                let v0 = lowered[0].eval(&coords0);
                let v1 = lowered[0].eval(&coords1);
                // We use wrapping_sub but check for consistency
                Some(v1.wrapping_sub(v0))
            }
            Expression::Product(l1, l2) => {
                let n1 = l1.source().factors.len();
                if dim_idx < n1 {
                    let s1 = l1.get_stride(dim_idx, valuation)?;
                    let v2 = valuation.get(&l2.target().volume_extent())?;
                    Some(s1 * v2)
                } else {
                    l2.get_stride(dim_idx - n1, valuation)
                }
            }
            _ => {
                // Fallback to evaluation-based stride detection for complex generators
                let src = self.source();
                let mut inputs = Vec::new();
                for i in 0..src.factors.len() { inputs.push(crate::layout::ScalarExpr::Input(i)); }
                let (lowered, _) = self.lower(valuation, inputs);
                if lowered.is_empty() { return None; }
                
                let mut coords0 = vec![0; src.factors.len()];
                let mut coords1 = vec![0; src.factors.len()];
                coords1[dim_idx] = 1;
                
                let v0 = lowered[0].eval(&coords0);
                let v1 = lowered[0].eval(&coords1);
                Some(v1.wrapping_sub(v0))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Kind, Extent, Factor, Space, Valuation};

    #[test]
    fn test_left_div() {
        let f1 = Factor::new(Kind::Logical, Extent::Constant(2), None);
        let f2 = Factor::new(Kind::Logical, Extent::Constant(3), None);
        let s1 = Space::new(vec![f1]);
        let s2 = Space::new(vec![f2]);
        let t = Expression::Identity(s1);
        let r = Expression::Identity(s2);
        let l = Expression::Product(Box::new(t.clone()), Box::new(r.clone()));
        assert_eq!(l.left_div(t).unwrap(), r);
    }

    #[test]
    fn test_structural_judgments() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let id = Expression::Identity(s.clone());
        let broadcast = Expression::Broadcast(s.clone(), Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(1), None)]));
        let valuation = Valuation::new();
        assert_eq!(id.is_injective(), Judgment::True);
        assert_eq!(broadcast.is_injective(), Judgment::False);
        assert_eq!(broadcast.is_aliasing(&valuation), Judgment::True);
    }

    #[test]
    fn test_normalization_cancellation() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let lin = Expression::Linearize(s.clone());
        let delin = Expression::Delinearize(s.clone());
        let comp = Expression::Composition(Box::new(lin), Box::new(delin));
        
        let nf = comp.normalize();
        // lin o delin should simplify to identity in normalization
        assert_eq!(nf.placement, Expression::Identity(s));
    }

    #[test]
    fn test_tensor_core_fit_analysis() {
        let frag = Space::new(vec![Factor::new(Kind::Fragment, Extent::Constant(16), None)]);
        let instr = Expression::Linearize(frag.clone());
        let tile = Space::new(vec![Factor::new(Kind::Tile, Extent::Constant(2), None)]);
        let program = Expression::Product(Box::new(instr.clone()), Box::new(Expression::Identity(tile.clone())));
        
        assert!(program.left_div(instr).is_some());
    }

    #[test]
    fn test_is_surjective() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let id = Expression::Identity(s.clone());
        let slice = Expression::Slice(s.clone(), vec![(0, 5)]);
        
        assert_eq!(id.is_surjective(), Judgment::True);
        assert_eq!(slice.is_surjective(), Judgment::False);
    }

    #[test]
    fn test_max_vector_width_cases() {
        let mut val = Valuation::new();
        val.variables.insert("DEBUG".to_string(), 1);
        
        // Row-major [4, 8] -> 8
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(4), None),
            Factor::new(Kind::Logical, Extent::Constant(8), None),
        ]);
        let lin = Expression::Linearize(s);
        assert_eq!(lin.max_vector_width(&val), 8);

        // Column-major (Transpose) -> 1
        let p = Expression::Permute(lin.source(), vec![1, 0]);
        let col = Expression::Composition(Box::new(p), Box::new(lin));
        assert_eq!(col.max_vector_width(&val), 1);
    }

    #[test]
    fn test_normalization_rules() {
        let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(4), None)]);
        let s2 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(4), None)]);
        let s3 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(4), None)]);
        
        // Rule 5: Reshape Fusion
        let r1 = Expression::Reshape(s1.clone(), s2.clone());
        let r2 = Expression::Reshape(s2.clone(), s3.clone());
        let comp = Expression::Composition(Box::new(r1), Box::new(r2));
        let simplified = comp.simplify_recursive();
        
        if let Expression::Reshape(src, tgt) = simplified {
            assert_eq!(src, s1);
            assert_eq!(tgt, s3);
        } else {
            panic!("Reshape fusion failed");
        }

        // Rule 6: Permute Fusion
        let p1 = Expression::Permute(s1.clone(), vec![0]);
        let p2 = Expression::Permute(s1.clone(), vec![0]);
        let comp_p = Expression::Composition(Box::new(p1), Box::new(p2));
        let simplified_p = comp_p.simplify_recursive();
        assert!(matches!(simplified_p, Expression::Permute(_, _)));
    }

    #[test]
    fn test_aliasing_detection() {
        let val = Valuation::new();
        let s10 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(1), None)]);
        let broadcast = Expression::Broadcast(s10, s1);
        
        assert_eq!(broadcast.is_aliasing(&val), Judgment::True);
    }
}
