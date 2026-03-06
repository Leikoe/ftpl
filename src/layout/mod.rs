use crate::core::{Extent, Factor, Kind, Space, Valuation};

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
    BitLinear(Box<ScalarExpr>, Vec<Vec<u8>>),
    And(Box<ScalarExpr>, Box<ScalarExpr>),
    Lt(Box<ScalarExpr>, Box<ScalarExpr>),
    Eq(Box<ScalarExpr>, Box<ScalarExpr>),
}

impl ScalarExpr {
    pub fn eval(&self, inputs: &[u64]) -> u64 {
        match self {
            ScalarExpr::Input(i) => inputs[*i],
            ScalarExpr::Constant(c) => *c,
            ScalarExpr::Add(a, b) => a.eval(inputs).wrapping_add(b.eval(inputs)),
            ScalarExpr::Mul(a, b) => a.eval(inputs).wrapping_mul(b.eval(inputs)),
            ScalarExpr::Div(a, b) => {
                let divisor = b.eval(inputs);
                if divisor == 0 {
                    0
                } else {
                    a.eval(inputs) / divisor
                }
            }
            ScalarExpr::Mod(a, b) => {
                let divisor = b.eval(inputs);
                if divisor == 0 {
                    0
                } else {
                    a.eval(inputs) % divisor
                }
            }
            ScalarExpr::Xor(a, b) => a.eval(inputs) ^ b.eval(inputs),
            ScalarExpr::BitShiftRight(a, s) => a.eval(inputs) >> s,
            ScalarExpr::BitLinear(a, m) => {
                let val = a.eval(inputs);
                let d = m.len();
                let mut out = 0;
                for i in 0..d {
                    let mut bit = 0;
                    for j in 0..d {
                        bit ^= ((val >> j) & 1) & (m[i][j] as u64);
                    }
                    out |= bit << i;
                }
                let mask = (1 << d) - 1;
                out | (val & !mask)
            }
            ScalarExpr::And(a, b) => {
                if a.eval(inputs) != 0 && b.eval(inputs) != 0 {
                    1
                } else {
                    0
                }
            }
            ScalarExpr::Lt(a, b) => {
                if a.eval(inputs) < b.eval(inputs) {
                    1
                } else {
                    0
                }
            }
            ScalarExpr::Eq(a, b) => {
                if a.eval(inputs) == b.eval(inputs) {
                    1
                } else {
                    0
                }
            }
        }
    }

    pub fn simplify(self) -> Self {
        match self {
            ScalarExpr::Add(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (ScalarExpr::Constant(0), _) => b,
                    (_, ScalarExpr::Constant(0)) => a,
                    (ScalarExpr::Constant(va), ScalarExpr::Constant(vb)) => {
                        ScalarExpr::Constant(va + vb)
                    }
                    (ScalarExpr::Add(inner_a, inner_b), ScalarExpr::Constant(c2)) => {
                        if let ScalarExpr::Constant(c1) = &**inner_b {
                            ScalarExpr::Add(inner_a.clone(), Box::new(ScalarExpr::Constant(c1 + c2)))
                                .simplify()
                        } else {
                            ScalarExpr::Add(Box::new(a), Box::new(b))
                        }
                    }
                    _ => ScalarExpr::Add(Box::new(a), Box::new(b)),
                }
            }
            ScalarExpr::Mul(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (ScalarExpr::Constant(1), _) => b,
                    (_, ScalarExpr::Constant(1)) => a,
                    (ScalarExpr::Constant(0), _) | (_, ScalarExpr::Constant(0)) => {
                        ScalarExpr::Constant(0)
                    }
                    (ScalarExpr::Constant(va), ScalarExpr::Constant(vb)) => {
                        ScalarExpr::Constant(va * vb)
                    }
                    _ => ScalarExpr::Mul(Box::new(a), Box::new(b)),
                }
            }
            ScalarExpr::Div(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (_, ScalarExpr::Constant(1)) => a,
                    (ScalarExpr::Constant(va), ScalarExpr::Constant(vb)) if *vb != 0 => {
                        ScalarExpr::Constant(va / vb)
                    }
                    (ScalarExpr::Add(l, r), ScalarExpr::Constant(k)) => {
                        match (l.as_ref(), r.as_ref()) {
                            (ScalarExpr::Mul(term, inner_r), rest) => {
                                if let ScalarExpr::Constant(coeff) = inner_r.as_ref() {
                                    if coeff == k {
                                        return ScalarExpr::Add(
                                            term.clone(),
                                            Box::new(ScalarExpr::Div(
                                                Box::new(rest.clone()),
                                                Box::new(ScalarExpr::Constant(*k)),
                                            )),
                                        )
                                        .simplify();
                                    }
                                }
                                ScalarExpr::Div(Box::new(a), Box::new(b))
                            }
                            (rest, ScalarExpr::Mul(term, inner_r)) => {
                                if let ScalarExpr::Constant(coeff) = inner_r.as_ref() {
                                    if coeff == k {
                                        return ScalarExpr::Add(
                                            term.clone(),
                                            Box::new(ScalarExpr::Div(
                                                Box::new(rest.clone()),
                                                Box::new(ScalarExpr::Constant(*k)),
                                            )),
                                        )
                                        .simplify();
                                    }
                                }
                                ScalarExpr::Div(Box::new(a), Box::new(b))
                            }
                            _ => ScalarExpr::Div(Box::new(a), Box::new(b)),
                        }
                    }
                    (_, ScalarExpr::Constant(d)) if d.is_power_of_two() => {
                        ScalarExpr::BitShiftRight(Box::new(a), d.trailing_zeros())
                    }
                    _ => ScalarExpr::Div(Box::new(a), Box::new(b)),
                }
            }
            ScalarExpr::Mod(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (ScalarExpr::Constant(va), ScalarExpr::Constant(vb)) if *vb != 0 => {
                        ScalarExpr::Constant(va % vb)
                    }
                    (ScalarExpr::Add(l, r), ScalarExpr::Constant(k)) => {
                        match (l.as_ref(), r.as_ref()) {
                            (ScalarExpr::Mul(_, inner_r), rest) => {
                                if let ScalarExpr::Constant(coeff) = inner_r.as_ref() {
                                    if coeff == k {
                                        return ScalarExpr::Mod(
                                            Box::new(rest.clone()),
                                            Box::new(ScalarExpr::Constant(*k)),
                                        )
                                        .simplify();
                                    }
                                }
                                ScalarExpr::Mod(Box::new(a), Box::new(b))
                            }
                            (rest, ScalarExpr::Mul(_, inner_r)) => {
                                if let ScalarExpr::Constant(coeff) = inner_r.as_ref() {
                                    if coeff == k {
                                        return ScalarExpr::Mod(
                                            Box::new(rest.clone()),
                                            Box::new(ScalarExpr::Constant(*k)),
                                        )
                                        .simplify();
                                    }
                                }
                                ScalarExpr::Mod(Box::new(a), Box::new(b))
                            }
                            _ => ScalarExpr::Mod(Box::new(a), Box::new(b)),
                        }
                    }
                    (ScalarExpr::Mul(_, inner_r), ScalarExpr::Constant(k)) => {
                        if let ScalarExpr::Constant(coeff) = inner_r.as_ref() {
                            if coeff == k {
                                ScalarExpr::Constant(0)
                            } else {
                                ScalarExpr::Mod(Box::new(a), Box::new(b))
                            }
                        } else {
                            ScalarExpr::Mod(Box::new(a), Box::new(b))
                        }
                    }
                    _ => ScalarExpr::Mod(Box::new(a), Box::new(b)),
                }
            }
            ScalarExpr::And(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                match (&a, &b) {
                    (ScalarExpr::Constant(0), _) | (_, ScalarExpr::Constant(0)) => {
                        ScalarExpr::Constant(0)
                    }
                    (ScalarExpr::Constant(_), _) => b,
                    (_, ScalarExpr::Constant(_)) => a,
                    _ => ScalarExpr::And(Box::new(a), Box::new(b)),
                }
            }
            ScalarExpr::BitLinear(a, m) => {
                let a = a.simplify();
                if let ScalarExpr::Constant(c) = a {
                    ScalarExpr::Constant(
                        ScalarExpr::BitLinear(Box::new(ScalarExpr::Constant(c)), m).eval(&[]),
                    )
                } else {
                    ScalarExpr::BitLinear(Box::new(a), m)
                }
            }
            _ => self,
        }
    }
}

pub fn invert_gf2(matrix: &[Vec<u8>]) -> Option<Vec<Vec<u8>>> {
    let n = matrix.len();
    let mut aug = vec![vec![0u8; 2 * n]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = matrix[i][j] & 1;
        }
        aug[i][i + n] = 1;
    }
    for i in 0..n {
        let mut pivot = i;
        while pivot < n && aug[pivot][i] == 0 {
            pivot += 1;
        }
        if pivot == n {
            return None;
        }
        aug.swap(i, pivot);
        for j in 0..n {
            if i != j && aug[j][i] == 1 {
                for k in 0..2 * n {
                    aug[j][k] ^= aug[i][k];
                }
            }
        }
    }
    let mut inv = vec![vec![0u8; n]; n];
    for i in 0..n {
        for j in 0..n {
            inv[i][j] = aug[i][j + n];
        }
    }
    Some(inv)
}

pub trait AsLayout {
    fn source(&self) -> Space;
    fn target(&self) -> Space;
    fn apply(&self, valuation: &Valuation, input: &[u64]) -> Option<Vec<u64>>;
    fn lower(&self, valuation: &Valuation, inputs: Vec<ScalarExpr>) -> (Vec<ScalarExpr>, ScalarExpr);
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
    Split(Space, usize, u64),
    Join(Space, usize),
    Squeeze(Space, usize),
    Unsqueeze(Space, usize, Factor),
    Repeat(Space, Space),
}

impl AsLayout for Expression {
    fn source(&self) -> Space {
        match self {
            Expression::Identity(s)
            | Expression::Linearize(s)
            | Expression::Permute(s, _)
            | Expression::Reshape(s, _)
            | Expression::BinaryShadow(s, _)
            | Expression::Slice(s, _)
            | Expression::Broadcast(s, _)
            | Expression::Pad(s, _, _)
            | Expression::Flip(s, _)
            | Expression::Split(s, _, _)
            | Expression::Join(s, _)
            | Expression::Squeeze(s, _)
            | Expression::Unsqueeze(s, _, _)
            | Expression::Repeat(s, _) => s.clone(),
            Expression::Delinearize(target) => Space::new(vec![Factor::new(
                Kind::Other("Offset".to_string()),
                target.volume_extent(),
                None,
            )]),
            Expression::Composition(l1, _) => l1.source(),
            Expression::Product(l1, l2) => l1.source().product(&l2.source()),
        }
    }

    fn target(&self) -> Space {
        match self {
            Expression::Identity(s)
            | Expression::Delinearize(s)
            | Expression::Flip(s, _)
            | Expression::BinaryShadow(s, _)
            | Expression::Repeat(_, s) => s.clone(),
            Expression::Linearize(s) => Space::new(vec![Factor::new(
                Kind::Other("Offset".to_string()),
                s.volume_extent(),
                None,
            )]),
            Expression::Permute(s, p) => {
                let mut factors =
                    vec![Factor::new(Kind::Logical, Extent::Constant(0), None); s.factors.len()];
                for (i, &pos) in p.iter().enumerate() {
                    factors[pos] = s.factors[i].clone();
                }
                Space::new(factors)
            }
            Expression::Composition(_, l2) => l2.target(),
            Expression::Product(l1, l2) => l1.target().product(&l2.target()),
            Expression::Reshape(_, t) | Expression::Broadcast(_, t) | Expression::Pad(_, t, _) => {
                t.clone()
            }
            Expression::Slice(s, ranges) => {
                let mut factors = Vec::new();
                for (i, f) in s.factors.iter().enumerate() {
                    let (start, end) = ranges[i];
                    factors.push(Factor::new(
                        f.kind.clone(),
                        Extent::Constant(end - start),
                        f.tag.0.clone(),
                    ));
                }
                Space::new(factors)
            }
            Expression::Split(s, idx, n1) => {
                let mut factors = s.factors.clone();
                let f = factors.remove(*idx);
                let n_total = if let Extent::Constant(v) = f.extent {
                    v
                } else {
                    1
                };
                factors.insert(
                    *idx,
                    Factor::new(f.kind.clone(), Extent::Constant(*n1), f.tag.0.clone()),
                );
                factors.insert(
                    idx + 1,
                    Factor::new(
                        f.kind.clone(),
                        Extent::Constant(n_total / *n1),
                        f.tag.0.clone(),
                    ),
                );
                Space::new(factors)
            }
            Expression::Join(s, idx) => {
                let mut factors = s.factors.clone();
                let f1 = factors.remove(*idx);
                let f2 = factors.remove(*idx);
                let new_extent = Extent::Product(vec![f1.extent, f2.extent]);
                factors.insert(*idx, Factor::new(f1.kind, new_extent, f1.tag.0));
                Space::new(factors)
            }
            Expression::Squeeze(s, idx) => {
                let mut factors = s.factors.clone();
                factors.remove(*idx);
                Space::new(factors)
            }
            Expression::Unsqueeze(s, idx, f) => {
                let mut factors = s.factors.clone();
                factors.insert(*idx, f.clone());
                Space::new(factors)
            }
        }
    }

    fn apply(&self, valuation: &Valuation, input: &[u64]) -> Option<Vec<u64>> {
        match self {
            Expression::Identity(_) => Some(input.to_vec()),
            Expression::Linearize(s) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut offset = 0;
                let mut stride = 1;
                for (f, &c) in s.factors.iter().rev().zip(input.iter().rev()) {
                    offset += c * stride;
                    stride *= valuation.get_extent(&f.extent)?;
                }
                Some(vec![offset])
            }
            Expression::Delinearize(s) => {
                if input.len() != 1 {
                    return None;
                }
                let mut offset = input[0];
                let mut output = vec![0; s.factors.len()];
                for (i, f) in s.factors.iter().enumerate().rev() {
                    let extent = valuation.get_extent(&f.extent)?;
                    output[i] = offset % extent;
                    offset /= extent;
                }
                if offset > 0 {
                    return None;
                }
                Some(output)
            }
            Expression::Permute(s, p) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
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
                if !s.is_valid(valuation, input) {
                    return None;
                }
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
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    let (start, end) = ranges[i];
                    if c < start || c >= end {
                        return None;
                    }
                    output.push(c - start);
                }
                Some(output)
            }
            Expression::Broadcast(s, target) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    let target_extent = valuation.get_extent(&target.factors[i].extent)?;
                    let source_extent = valuation.get_extent(&s.factors[i].extent)?;
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
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    if dims[i] {
                        let n = valuation.get_extent(&s.factors[i].extent)?;
                        output.push(n - 1 - c);
                    } else {
                        output.push(c);
                    }
                }
                Some(output)
            }
            Expression::Split(s, idx, n1) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = input.to_vec();
                let val = output.remove(*idx);
                output.insert(*idx, val / *n1);
                output.insert(*idx + 1, val % *n1);
                Some(output)
            }
            Expression::Join(s, idx) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = input.to_vec();
                let v1 = output.remove(*idx);
                let v2 = output.remove(*idx);
                let n2 = valuation.get_extent(&s.factors[*idx + 1].extent)?;
                output.insert(*idx, v1 * n2 + v2);
                Some(output)
            }
            Expression::Squeeze(s, idx) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = input.to_vec();
                output.remove(*idx);
                Some(output)
            }
            Expression::Unsqueeze(s, idx, _) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = input.to_vec();
                output.insert(*idx, 0);
                Some(output)
            }
            Expression::Repeat(s, target) => {
                if !s.is_valid(valuation, input) {
                    return None;
                }
                let mut output = Vec::new();
                for (i, &c) in input.iter().enumerate() {
                    let se = valuation.get_extent(&s.factors[i].extent)?;
                    let te = valuation.get_extent(&target.factors[i].extent)?;
                    if te < se {
                        output.push(c % te);
                    } else {
                        output.push(c);
                    }
                }
                Some(output)
            }
        }
    }

    fn lower(&self, valuation: &Valuation, inputs: Vec<ScalarExpr>) -> (Vec<ScalarExpr>, ScalarExpr) {
        let mut domain = ScalarExpr::Constant(1);

        if inputs.len() != self.source().factors.len() {
            return (vec![], ScalarExpr::Constant(0));
        }

        for (i, f) in self.source().factors.iter().enumerate() {
            if let Some(extent) = valuation.get_extent(&f.extent) {
                let in_bounds = ScalarExpr::Lt(
                    Box::new(inputs[i].clone()),
                    Box::new(ScalarExpr::Constant(extent)),
                );
                domain = ScalarExpr::And(Box::new(domain), Box::new(in_bounds));
            }
        }

        match self {
            Expression::Identity(_) => (inputs, domain),
            Expression::Linearize(s) => {
                let mut offset = ScalarExpr::Constant(0);
                let mut stride = 1;
                for (i, f) in s.factors.iter().enumerate().rev() {
                    let extent = valuation.get_extent(&f.extent).unwrap_or(1);
                    let term = ScalarExpr::Mul(
                        Box::new(inputs[i].clone()),
                        Box::new(ScalarExpr::Constant(stride)),
                    );
                    offset = ScalarExpr::Add(Box::new(offset), Box::new(term));
                    stride *= extent;
                }
                (vec![offset.simplify()], domain)
            }
            Expression::Delinearize(s) => {
                if inputs.is_empty() {
                    return (vec![], ScalarExpr::Constant(0));
                }
                let mut offset = inputs[0].clone();
                let mut output = vec![ScalarExpr::Constant(0); s.factors.len()];
                for (i, f) in s.factors.iter().enumerate().rev() {
                    let extent = valuation.get_extent(&f.extent).unwrap_or(1);
                    output[i] = ScalarExpr::Mod(
                        Box::new(offset.clone()),
                        Box::new(ScalarExpr::Constant(extent)),
                    )
                    .simplify();
                    offset =
                        ScalarExpr::Div(Box::new(offset), Box::new(ScalarExpr::Constant(extent)))
                            .simplify();
                }
                if let Some(vol) = valuation.get_extent(&s.volume_extent()) {
                    let in_vol = ScalarExpr::Lt(
                        Box::new(inputs[0].clone()),
                        Box::new(ScalarExpr::Constant(vol)),
                    );
                    domain = ScalarExpr::And(Box::new(domain), Box::new(in_vol));
                }
                (output, domain)
            }
            Expression::Permute(s, p) => {
                let mut output = vec![ScalarExpr::Constant(0); s.factors.len()];
                for (i, &pos) in p.iter().enumerate() {
                    if i < inputs.len() {
                        output[pos] = inputs[i].clone();
                    }
                }
                (output, domain)
            }
            Expression::Composition(l1, l2) => {
                let (mid, d1) = l1.lower(valuation, inputs);
                let (out, d2) = l2.lower(valuation, mid);
                (out, ScalarExpr::And(Box::new(d1), Box::new(d2)).simplify())
            }
            Expression::Product(l1, l2) => {
                let n1 = l1.source().factors.len();
                let (i1, i2) = inputs.split_at(n1);
                let (mut o1, d1) = l1.lower(valuation, i1.to_vec());
                let (mut o2, d2) = l2.lower(valuation, i2.to_vec());
                o1.append(&mut o2);
                (o1, ScalarExpr::And(Box::new(d1), Box::new(d2)).simplify())
            }
            Expression::Reshape(s, t) => {
                let (linearized, d1) = Expression::Linearize(s.clone()).lower(valuation, inputs);
                let (output, d2) = Expression::Delinearize(t.clone()).lower(valuation, linearized);
                (
                    output,
                    ScalarExpr::And(Box::new(d1), Box::new(d2)).simplify(),
                )
            }
            Expression::Split(_, idx, n1) => {
                let mut output = inputs;
                let val = output.remove(*idx);
                output.insert(
                    *idx,
                    ScalarExpr::Div(Box::new(val.clone()), Box::new(ScalarExpr::Constant(*n1)))
                        .simplify(),
                );
                output.insert(
                    *idx + 1,
                    ScalarExpr::Mod(Box::new(val), Box::new(ScalarExpr::Constant(*n1))).simplify(),
                );
                (output, domain)
            }
            Expression::Join(s, idx) => {
                let mut output = inputs;
                let v1 = output.remove(*idx);
                let v2 = output.remove(*idx);
                let n2 = valuation
                    .get_extent(&s.factors[*idx + 1].extent)
                    .unwrap_or(1);
                output.insert(
                    *idx,
                    ScalarExpr::Add(
                        Box::new(ScalarExpr::Mul(
                            Box::new(v1),
                            Box::new(ScalarExpr::Constant(n2)),
                        )),
                        Box::new(v2),
                    )
                    .simplify(),
                );
                (output, domain)
            }
            Expression::Squeeze(_, idx) => {
                let mut output = inputs;
                output.remove(*idx);
                (output, domain)
            }
            Expression::Unsqueeze(_, idx, _) => {
                let mut output = inputs;
                output.insert(*idx, ScalarExpr::Constant(0));
                (output, domain)
            }
            Expression::Repeat(s, target) => {
                let mut output = Vec::new();
                for (i, expr) in inputs.into_iter().enumerate() {
                    let se = valuation.get_extent(&s.factors[i].extent).unwrap_or(1);
                    let te = valuation.get_extent(&target.factors[i].extent).unwrap_or(1);
                    if te < se {
                        output.push(
                            ScalarExpr::Mod(Box::new(expr), Box::new(ScalarExpr::Constant(te)))
                                .simplify(),
                        );
                    } else {
                        output.push(expr);
                    }
                }
                (output, domain)
            }
            Expression::BinaryShadow(s, matrix) => {
                let (linearized, d) = Expression::Linearize(s.clone()).lower(valuation, inputs);
                let offset = linearized[0].clone();
                let bit_linear = ScalarExpr::BitLinear(Box::new(offset), matrix.clone());
                let (output, d2) =
                    Expression::Delinearize(s.clone()).lower(valuation, vec![bit_linear]);
                (
                    output,
                    ScalarExpr::And(Box::new(d), Box::new(d2)).simplify(),
                )
            }
            Expression::Slice(_, ranges) => {
                let mut output = Vec::new();
                let mut slice_domain = domain;
                for (i, c) in inputs.into_iter().enumerate() {
                    let (start, end) = ranges[i];
                    let in_slice = ScalarExpr::And(
                        Box::new(ScalarExpr::And(
                            Box::new(ScalarExpr::Constant(1)),
                            Box::new(ScalarExpr::Lt(
                                Box::new(ScalarExpr::Constant(if start > 0 {
                                    start - 1
                                } else {
                                    0
                                })),
                                Box::new(c.clone()),
                            )),
                        )),
                        Box::new(ScalarExpr::Lt(
                            Box::new(c.clone()),
                            Box::new(ScalarExpr::Constant(end)),
                        )),
                    );
                    slice_domain = ScalarExpr::And(Box::new(slice_domain), Box::new(in_slice));
                    output.push(
                        ScalarExpr::Add(
                            Box::new(c),
                            Box::new(ScalarExpr::Constant(0u64.wrapping_sub(start))),
                        )
                        .simplify(),
                    );
                }
                (output, slice_domain.simplify())
            }
            Expression::Broadcast(s, target) => {
                let mut output = Vec::new();
                for (i, expr) in inputs.into_iter().enumerate() {
                    let se = valuation.get_extent(&s.factors[i].extent).unwrap_or(1);
                    let te = valuation.get_extent(&target.factors[i].extent).unwrap_or(1);
                    if se > 1 && te == 1 {
                        output.push(ScalarExpr::Constant(0));
                    } else {
                        output.push(expr);
                    }
                }
                (output, domain)
            }
            Expression::Pad(_s, _, padding) => {
                let mut output = Vec::new();
                for (i, expr) in inputs.into_iter().enumerate() {
                    let (left, _) = padding[i];
                    output.push(
                        ScalarExpr::Add(Box::new(expr), Box::new(ScalarExpr::Constant(left)))
                            .simplify(),
                    );
                }
                (output, domain)
            }
            Expression::Flip(s, dims) => {
                let mut output = Vec::new();
                for (i, expr) in inputs.into_iter().enumerate() {
                    if dims[i] {
                        let n = valuation.get_extent(&s.factors[i].extent).unwrap_or(1);
                        output.push(
                            ScalarExpr::Add(
                                Box::new(ScalarExpr::Constant(n - 1)),
                                Box::new(ScalarExpr::Mul(
                                    Box::new(ScalarExpr::Constant(u64::MAX)),
                                    Box::new(expr),
                                )),
                            )
                            .simplify(),
                        );
                    } else {
                        output.push(expr);
                    }
                }
                (output, domain)
            }
        }
    }
}

/// A high-level, production-grade Layout object.
/// Encapsulates an Expression and provides fluent builder methods.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub expr: Expression,
}

impl Layout {
    pub fn new(expr: Expression) -> Self {
        Self { expr }
    }

    // --- Constructors ---

    pub fn row_major<S: Into<Space>>(space: S) -> Self {
        Self::new(Expression::Linearize(space.into()))
    }

    pub fn col_major<S: Into<Space>>(space: S) -> Self {
        let space = space.into();
        let p = (0..space.rank()).rev().collect::<Vec<_>>();
        let transposed_space = space.permute(&p);
        Self::new(Expression::Composition(
            Box::new(Expression::Permute(space, p)),
            Box::new(Expression::Linearize(transposed_space)),
        ))
    }

    pub fn identity<S: Into<Space>>(space: S) -> Self {
        Self::new(Expression::Identity(space.into()))
    }

    // --- Fluent Builder Methods (No-Movement Views) ---

    pub fn transpose(self) -> Self {
        let rank = self.source().rank();
        if rank < 2 {
            return self;
        }
        let mut p: Vec<usize> = (0..rank).collect();
        p.swap(rank - 1, rank - 2);
        self.permute(p)
    }

    pub fn permute(self, p: Vec<usize>) -> Self {
        let src = self.source();
        // New view V: PermutedSrc -> src
        // We need to construct the space that, when permuted by p, gives src.
        // Or simpler: Permute(src, p) maps src -> PermutedTarget.
        // If the user wants to "permute" the current layout, they usually mean
        // they want the new layout to have a permuted source.
        let new_src = src.permute(&p); 
        Self::new(Expression::Composition(
            Box::new(Expression::Permute(new_src, p)),
            Box::new(self.expr),
        ))
    }

    pub fn reshape<S: Into<Space>>(self, new_src: S) -> Self {
        let new_src = new_src.into();
        let current_src = self.source();
        Self::new(Expression::Composition(
            Box::new(Expression::Reshape(new_src, current_src)),
            Box::new(self.expr),
        ))
    }

    pub fn split(self, dim_idx: usize, n1: u64) -> Self {
        // current L: A -> B. We want L': Split(A) -> B.
        // So L' = L o Join(Split(A))? No.
        // If we split a dimension in the source, the new source is larger rank.
        let current_src = self.source();
        let split_view = Expression::Split(current_src, dim_idx, n1);
        let new_src = split_view.target();
        Self::new(Expression::Composition(
            Box::new(split_view),
            Box::new(self.expr)
        ))
    }

    pub fn join(self, dim_idx: usize) -> Self {
        let src = self.source();
        Self::new(Expression::Composition(
            Box::new(Expression::Join(src, dim_idx)),
            Box::new(self.expr),
        ))
    }

    pub fn swizzle(self, matrix: Vec<Vec<u8>>) -> Self {
        let tgt = self.target();
        Self::new(Expression::Composition(
            Box::new(self.expr),
            Box::new(Expression::BinaryShadow(tgt, matrix)),
        ))
    }

    // --- Algebraic Ops ---

    pub fn compose(self, other: Layout) -> Self {
        Self::new(Expression::Composition(
            Box::new(self.expr),
            Box::new(other.expr),
        ))
    }

    pub fn product(self, other: Layout) -> Self {
        Self::new(Expression::Product(
            Box::new(self.expr),
            Box::new(other.expr),
        ))
    }
}

impl Layout {
    pub fn inverse(&self) -> Option<Self> {
        self.expr.inverse().map(Self::new)
    }

    // Forwarding analysis logic (defined in analysis/mod.rs via impl Expression)
    pub fn is_injective(&self) -> crate::analysis::Judgment {
        self.expr.is_injective()
    }

    pub fn is_surjective(&self) -> crate::analysis::Judgment {
        self.expr.is_surjective()
    }

    pub fn is_aliasing(&self, valuation: &Valuation) -> crate::analysis::Judgment {
        self.expr.is_aliasing(valuation)
    }

    pub fn max_vector_width(&self, valuation: &Valuation) -> u64 {
        self.expr.max_vector_width(valuation)
    }

    pub fn equivalent_to(&self, other: &Layout) -> bool {
        self.expr.equivalent_to(&other.expr)
    }

    pub fn shuffle_to(&self, target: &Layout) -> Option<Self> {
        self.expr.shuffle_to(&target.expr).map(Self::new)
    }

    pub fn bank_conflict_strides(&self, valuation: &Valuation) -> Vec<(usize, u64)> {
        self.expr.bank_conflict_strides(valuation)
    }
}

impl AsLayout for Layout {
    fn source(&self) -> Space {
        self.expr.source()
    }
    fn target(&self) -> Space {
        self.expr.target()
    }
    fn apply(&self, valuation: &Valuation, input: &[u64]) -> Option<Vec<u64>> {
        self.expr.apply(valuation, input)
    }
    fn lower(&self, valuation: &Valuation, inputs: Vec<ScalarExpr>) -> (Vec<ScalarExpr>, ScalarExpr) {
        self.expr.lower(valuation, inputs)
    }
}

impl Expression {
    pub fn inverse(&self) -> Option<Expression> {
        match self {
            Expression::Identity(s) => Some(Expression::Identity(s.clone())),
            Expression::Linearize(s) => Some(Expression::Delinearize(s.clone())),
            Expression::Delinearize(s) => Some(Expression::Linearize(s.clone())),
            Expression::Permute(_s, p) => {
                let mut inv_p = vec![0; p.len()];
                for (i, &pos) in p.iter().enumerate() {
                    inv_p[pos] = i;
                }
                Some(Expression::Permute(self.target(), inv_p))
            }
            Expression::Composition(l1, l2) => {
                let inv1 = l1.inverse()?;
                let inv2 = l2.inverse()?;
                Some(Expression::Composition(Box::new(inv1), Box::new(inv2)))
            }
            Expression::Product(l1, l2) => {
                let inv1 = l1.inverse()?;
                let inv2 = l2.inverse()?;
                Some(Expression::Product(Box::new(inv1), Box::new(inv2)))
            }
            Expression::Reshape(s, t) => Some(Expression::Reshape(t.clone(), s.clone())),
            Expression::BinaryShadow(s, m) => {
                let inv_m = invert_gf2(m)?;
                Some(Expression::BinaryShadow(s.clone(), inv_m))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Extent, Factor, Kind, Space, Valuation};

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
        let t = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(6), None)]);
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
        let (lowered, domain) = lin.lower(&valuation, inputs);
        assert_eq!(lowered[0].eval(&[1, 2]), 5);
        assert_eq!(domain.eval(&[1, 2]), 1); // Valid
        assert_eq!(domain.eval(&[2, 0]), 0); // Invalid (H=2)
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
        let l1 = Expression::Identity(Space::new(vec![Factor::new(
            Kind::Logical,
            Extent::Constant(2),
            None,
        )]));
        let l2 = Expression::Identity(Space::new(vec![Factor::new(
            Kind::Logical,
            Extent::Constant(3),
            None,
        )]));
        let prod = Expression::Product(Box::new(l1), Box::new(l2));
        let val = Valuation::new();
        assert_eq!(prod.apply(&val, &[1, 2]).unwrap(), vec![1, 2]);
    }

    #[test]
    fn test_scalar_expr_simplification() {
        // (x * 1) + 0 -> x
        let expr = ScalarExpr::Add(
            Box::new(ScalarExpr::Mul(
                Box::new(ScalarExpr::Input(0)),
                Box::new(ScalarExpr::Constant(1)),
            )),
            Box::new(ScalarExpr::Constant(0)),
        )
        .simplify();
        assert_eq!(expr, ScalarExpr::Input(0));

        // x / 4 -> x >> 2
        let div = ScalarExpr::Div(
            Box::new(ScalarExpr::Input(0)),
            Box::new(ScalarExpr::Constant(4)),
        )
        .simplify();
        assert_eq!(
            div,
            ScalarExpr::BitShiftRight(Box::new(ScalarExpr::Input(0)), 2)
        );
    }
}
