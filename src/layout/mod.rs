use crate::core::{Space, Valuation, Kind, Factor, Extent};

/// A partial typed layout `L : A ⇀ B`.
pub trait Layout {
    fn source(&self) -> Space;
    fn target(&self) -> Space;

    /// Maps a product coordinate in source space to target space.
    /// Returns `None` if the input is outside the validity domain.
    fn apply(&self, valuation: &Valuation, input: &[u64]) -> Option<Vec<u64>>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarExpr {
    Input(usize),
    Constant(u64),
    Add(Box<ScalarExpr>, Box<ScalarExpr>),
    Mul(Box<ScalarExpr>, Box<ScalarExpr>),
    Div(Box<ScalarExpr>, Box<ScalarExpr>),
    Mod(Box<ScalarExpr>, Box<ScalarExpr>),
    Xor(Box<ScalarExpr>, Box<ScalarExpr>),
    BitShiftRight(Box<ScalarExpr>, u32),
}

impl ScalarExpr {
    pub fn eval(&self, inputs: &[u64]) -> u64 {
        match self {
            ScalarExpr::Input(i) => inputs[*i],
            ScalarExpr::Constant(c) => *c,
            ScalarExpr::Add(a, b) => a.eval(inputs) + b.eval(inputs),
            ScalarExpr::Mul(a, b) => a.eval(inputs) * b.eval(inputs),
            ScalarExpr::Div(a, b) => a.eval(inputs) / b.eval(inputs),
            ScalarExpr::Mod(a, b) => a.eval(inputs) % b.eval(inputs),
            ScalarExpr::Xor(a, b) => a.eval(inputs) ^ b.eval(inputs),
            ScalarExpr::BitShiftRight(a, s) => a.eval(inputs) >> s,
        }
    }

    pub fn simplify(self) -> Self {
        match self {
            ScalarExpr::Div(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (_, ScalarExpr::Constant(d)) if d.is_power_of_two() => {
                        let shift = d.trailing_zeros();
                        ScalarExpr::BitShiftRight(Box::new(a), shift)
                    }
                    (_, ScalarExpr::Constant(1)) => a,
                    _ => ScalarExpr::Div(Box::new(a), Box::new(b)),
                }
            }
            ScalarExpr::Add(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (ScalarExpr::Constant(0), _) => b,
                    (_, ScalarExpr::Constant(0)) => a,
                    _ => ScalarExpr::Add(Box::new(a), Box::new(b)),
                }
            }
            ScalarExpr::Mul(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (ScalarExpr::Constant(1), _) => b,
                    (_, ScalarExpr::Constant(1)) => a,
                    (ScalarExpr::Constant(0), _) | (_, ScalarExpr::Constant(0)) => ScalarExpr::Constant(0),
                    _ => ScalarExpr::Mul(Box::new(a), Box::new(b)),
                }
            }
            _ => self,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identity(Space),
    Linearize(Space),
    Delinearize(Space),
    Permute(Space, Vec<usize>),
    Composition(Box<Expression>, Box<Expression>),
    Product(Box<Expression>, Box<Expression>),
    Reshape(Space, Space),
    BinaryShadow(Space, Vec<Vec<u8>>),
    Slice(Space, Vec<(u64, u64)>),
    Broadcast(Space, Space),
    Pad(Space, Space, Vec<(u64, u64)>),
    Flip(Space, Vec<bool>),
}

impl Layout for Expression {
    fn source(&self) -> Space {
        match self {
            Expression::Identity(s) | Expression::Linearize(s) | Expression::Permute(s, _) |
            Expression::Reshape(s, _) | Expression::BinaryShadow(s, _) | Expression::Slice(s, _) |
            Expression::Broadcast(s, _) | Expression::Pad(s, _, _) | Expression::Flip(s, _) => s.clone(),
            Expression::Delinearize(target) => {
                Space::new(vec![Factor::new(Kind::Other("Offset".to_string()), target.volume_extent(), None)])
            }
            Expression::Composition(l1, _) => l1.source(),
            Expression::Product(l1, l2) => l1.source().product(&l2.source()),
        }
    }

    fn target(&self) -> Space {
        match self {
            Expression::Identity(s) | Expression::Delinearize(s) | Expression::Flip(s, _) => s.clone(),
            Expression::Linearize(s) => {
                Space::new(vec![Factor::new(Kind::Other("Offset".to_string()), s.volume_extent(), None)])
            }
            Expression::Permute(s, p) => {
                let mut factors = vec![Factor::new(Kind::Logical, Extent::Constant(0), None); s.factors.len()];
                for (i, &pos) in p.iter().enumerate() {
                    factors[pos] = s.factors[i].clone();
                }
                Space::new(factors)
            }
            Expression::Composition(_, l2) => l2.target(),
            Expression::Product(l1, l2) => l1.target().product(&l2.target()),
            Expression::Reshape(_, t) | Expression::Broadcast(_, t) | Expression::Pad(_, t, _) => t.clone(),
            Expression::Slice(s, ranges) => {
                let mut factors = Vec::new();
                for (i, f) in s.factors.iter().enumerate() {
                    let (start, end) = ranges[i];
                    factors.push(Factor::new(f.kind.clone(), Extent::Constant(end - start), f.tag.0.clone()));
                }
                Space::new(factors)
            }
            Expression::BinaryShadow(s, _) => s.clone(),
        }
    }

    fn apply(&self, valuation: &Valuation, input: &[u64]) -> Option<Vec<u64>> {
        match self {
            Expression::Identity(_) => Some(input.to_vec()),
            Expression::Linearize(s) => {
                if !s.is_valid(valuation, input) { return None; }
                let mut offset = 0;
                let mut stride = 1;
                for (f, &c) in s.factors.iter().rev().zip(input.iter().rev()) {
                    offset += c * stride;
                    stride *= valuation.get(&f.extent)?;
                }
                Some(vec![offset])
            }
            Expression::Delinearize(s) => {
                if input.len() != 1 { return None; }
                let mut offset = input[0];
                let mut output = vec![0; s.factors.len()];
                for (i, f) in s.factors.iter().enumerate().rev() {
                    let extent = valuation.get(&f.extent)?;
                    output[i] = offset % extent;
                    offset /= extent;
                }
                if offset > 0 { return None; }
                Some(output)
            }
            Expression::Permute(s, p) => {
                if !s.is_valid(valuation, input) { return None; }
                let mut output = vec![0; s.factors.len()];
                for (i, &pos) in p.iter().enumerate() {
                    output[pos] = input[i];
                }
                Some(output)
            }
            Expression::Composition(l1, l2) => {
                let mid = l1.apply(valuation, input)?;
                l2.apply(valuation, &mid)
            }
            Expression::Product(l1, l2) => {
                let n1 = l1.source().factors.len();
                let (i1, i2) = input.split_at(n1);
                let mut o1 = l1.apply(valuation, i1)?;
                let mut o2 = l2.apply(valuation, i2)?;
                o1.append(&mut o2);
                Some(o1)
            }
            Expression::Reshape(s, t) => {
                let offset = Expression::Linearize(s.clone()).apply(valuation, input)?[0];
                Expression::Delinearize(t.clone()).apply(valuation, &[offset])
            }
            Expression::BinaryShadow(s, matrix) => {
                if !s.is_valid(valuation, input) { return None; }
                let offset = Expression::Linearize(s.clone()).apply(valuation, input)?[0];
                let d = matrix.len();
                let mut new_offset = 0;
                for i in 0..d {
                    let mut bit = 0;
                    for j in 0..d {
                        bit ^= ((offset >> j) & 1) & (matrix[i][j] as u64);
                    }
                    new_offset |= bit << i;
                }
                let mask = (1 << d) - 1;
                new_offset |= offset & !mask;
                Expression::Delinearize(s.clone()).apply(valuation, &[new_offset])
            }
            Expression::Slice(s, ranges) => {
                if !s.is_valid(valuation, input) { return None; }
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    let (start, end) = ranges[i];
                    if c < start || c >= end { return None; }
                    output.push(c - start);
                }
                Some(output)
            }
            Expression::Broadcast(s, target) => {
                if !s.is_valid(valuation, input) { return None; }
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    let target_extent = valuation.get(&target.factors[i].extent)?;
                    let source_extent = valuation.get(&s.factors[i].extent)?;
                    if source_extent > 1 && target_extent == 1 {
                        output.push(0);
                    } else {
                        output.push(c);
                    }
                }
                Some(output)
            }
            Expression::Pad(_, _, padding) => {
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    let (left, _) = padding[i];
                    output.push(c + left);
                }
                Some(output)
            }
            Expression::Flip(s, dims) => {
                if !s.is_valid(valuation, input) { return None; }
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    if dims[i] {
                        let n = valuation.get(&s.factors[i].extent)?;
                        output.push(n - 1 - c);
                    } else {
                        output.push(c);
                    }
                }
                Some(output)
            }
        }
    }
}

impl Expression {
    pub fn lower(&self, valuation: &Valuation, inputs: Vec<ScalarExpr>) -> Vec<ScalarExpr> {
        match self {
            Expression::Identity(_) => inputs,
            Expression::Linearize(s) => {
                let mut offset = ScalarExpr::Constant(0);
                let mut stride = 1;
                for (i, f) in s.factors.iter().enumerate().rev() {
                    let extent = valuation.get(&f.extent).unwrap_or(1);
                    let term = ScalarExpr::Mul(
                        Box::new(inputs[i].clone()),
                        Box::new(ScalarExpr::Constant(stride))
                    );
                    offset = ScalarExpr::Add(Box::new(offset), Box::new(term));
                    stride *= extent;
                }
                vec![offset.simplify()]
            }
            Expression::Delinearize(s) => {
                if inputs.is_empty() { return inputs; }
                let mut offset = inputs[0].clone();
                let mut output = vec![ScalarExpr::Constant(0); s.factors.len()];
                for (i, f) in s.factors.iter().enumerate().rev() {
                    let extent = valuation.get(&f.extent).unwrap_or(1);
                    output[i] = ScalarExpr::Mod(Box::new(offset.clone()), Box::new(ScalarExpr::Constant(extent))).simplify();
                    offset = ScalarExpr::Div(Box::new(offset), Box::new(ScalarExpr::Constant(extent))).simplify();
                }
                output
            }
            Expression::Permute(s, p) => {
                let mut output = vec![ScalarExpr::Constant(0); s.factors.len()];
                for (i, &pos) in p.iter().enumerate() {
                    if i < inputs.len() { output[pos] = inputs[i].clone(); }
                }
                output
            }
            Expression::Reshape(s, t) => {
                let linearized = Expression::Linearize(s.clone()).lower(valuation, inputs);
                Expression::Delinearize(t.clone()).lower(valuation, linearized)
            }
            Expression::Composition(l1, l2) => {
                let mid = l1.lower(valuation, inputs);
                l2.lower(valuation, mid)
            }
            Expression::Product(l1, l2) => {
                let n1 = l1.source().factors.len();
                let (i1, i2) = inputs.split_at(n1);
                let mut o1 = l1.lower(valuation, i1.to_vec());
                let mut o2 = l2.lower(valuation, i2.to_vec());
                o1.append(&mut o2);
                o1
            }
            _ => inputs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Kind, Extent, Factor, Space, Valuation};

    #[test]
    fn test_identity_layout() {
        let f1 = Factor::new(Kind::Logical, Extent::Constant(10), None);
        let space = Space::new(vec![f1.clone()]);
        let id = Expression::Identity(space);
        let valuation = Valuation::new();
        let input = vec![5];
        let output = id.apply(&valuation, &input).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn test_reshape() {
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(2), None),
            Factor::new(Kind::Logical, Extent::Constant(3), None),
        ]);
        let t = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(6), None),
        ]);
        let reshape = Expression::Reshape(s, t);
        let valuation = Valuation::new();
        assert_eq!(reshape.apply(&valuation, &[0, 0]).unwrap(), vec![0]);
        assert_eq!(reshape.apply(&valuation, &[1, 2]).unwrap(), vec![5]);
    }

    #[test]
    fn test_binary_shadow() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(4), None)]);
        let matrix = vec![vec![0, 1], vec![1, 0]];
        let shadow = Expression::BinaryShadow(s, matrix);
        let valuation = Valuation::new();
        assert_eq!(shadow.apply(&valuation, &[1]).unwrap(), vec![2]);
        assert_eq!(shadow.apply(&valuation, &[2]).unwrap(), vec![1]);
    }

    #[test]
    fn test_broadcast() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let t = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(1), None)]);
        let broadcast = Expression::Broadcast(s, t);
        let valuation = Valuation::new();
        assert_eq!(broadcast.apply(&valuation, &[5]).unwrap(), vec![0]);
    }

    #[test]
    fn test_lowering_complex() {
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(2), None),
            Factor::new(Kind::Logical, Extent::Constant(3), None),
        ]);
        let lin = Expression::Linearize(s);
        let valuation = Valuation::new();
        let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
        let lowered = lin.lower(&valuation, inputs);
        assert_eq!(lowered[0].eval(&[1, 2]), 5);
    }

    #[test]
    fn test_permute_mapping() {
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(2), None),
            Factor::new(Kind::Logical, Extent::Constant(3), None),
        ]);
        let p = Expression::Permute(s, vec![1, 0]);
        let val = Valuation::new();
        assert_eq!(p.apply(&val, &[1, 2]).unwrap(), vec![2, 1]);
    }

    #[test]
    fn test_slice_mapping() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let slice = Expression::Slice(s, vec![(2, 5)]); // [2, 5)
        let val = Valuation::new();
        assert_eq!(slice.apply(&val, &[2]).unwrap(), vec![0]);
        assert_eq!(slice.apply(&val, &[4]).unwrap(), vec![2]);
        assert!(slice.apply(&val, &[5]).is_none());
    }

    #[test]
    fn test_pad_mapping() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let t = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(12), None)]);
        let pad = Expression::Pad(s, t, vec![(1, 1)]);
        let val = Valuation::new();
        assert_eq!(pad.apply(&val, &[0]).unwrap(), vec![1]);
        assert_eq!(pad.apply(&val, &[9]).unwrap(), vec![10]);
    }

    #[test]
    fn test_flip_mapping() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let flip = Expression::Flip(s, vec![true]);
        let val = Valuation::new();
        assert_eq!(flip.apply(&val, &[0]).unwrap(), vec![9]);
        assert_eq!(flip.apply(&val, &[9]).unwrap(), vec![0]);
    }

    #[test]
    fn test_linearize_delinearize_inverse() {
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(2), None),
            Factor::new(Kind::Logical, Extent::Constant(3), None),
        ]);
        let lin = Expression::Linearize(s.clone());
        let delin = Expression::Delinearize(s);
        let val = Valuation::new();
        
        let coord = vec![1, 2];
        let offset = lin.apply(&val, &coord).unwrap();
        let back = delin.apply(&val, &offset).unwrap();
        assert_eq!(coord, back);
    }

    #[test]
    fn test_layout_product() {
        let l1 = Expression::Identity(Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(2), None)]));
        let l2 = Expression::Identity(Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(3), None)]));
        let prod = Expression::Product(Box::new(l1), Box::new(l2));
        let val = Valuation::new();
        assert_eq!(prod.apply(&val, &[1, 2]).unwrap(), vec![1, 2]);
    }

    #[test]
    fn test_scalar_expr_simplification() {
        // (x * 1) + 0 -> x
        let expr = ScalarExpr::Add(
            Box::new(ScalarExpr::Mul(Box::new(ScalarExpr::Input(0)), Box::new(ScalarExpr::Constant(1)))),
            Box::new(ScalarExpr::Constant(0))
        ).simplify();
        assert_eq!(expr, ScalarExpr::Input(0));

        // x / 4 -> x >> 2
        let div = ScalarExpr::Div(Box::new(ScalarExpr::Input(0)), Box::new(ScalarExpr::Constant(4))).simplify();
        assert_eq!(div, ScalarExpr::BitShiftRight(Box::new(ScalarExpr::Input(0)), 2));
    }
}
