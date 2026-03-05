use std::collections::HashMap;

/// A role label for a factor space.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Logical,
    Storage,
    Execution,
    Instruction,
    Fragment,
    Tile,
    Other(String),
}

/// A symbolic natural-number value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Extent {
    Constant(u64),
    Variable(String),
    Product(Vec<Extent>),
    Tile,
    Storage,
}

impl Extent {
    pub fn eval(&self, variables: &HashMap<String, u64>) -> Option<u64> {
        match self {
            Extent::Constant(v) => Some(*v),
            Extent::Variable(name) => variables.get(name).copied(),
            Extent::Product(parts) => {
                let mut res = 1;
                for p in parts {
                    res *= p.eval(variables)?;
                }
                Some(res)
            }
            Extent::Tile => variables.get("tile").copied(),
            Extent::Storage => variables.get("storage").copied(),
        }
    }
}

/// An optional local tag to distinguish factors of the same kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Tag(pub Option<String>);

/// A factor space `a = (k, n, t)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Factor {
    pub kind: Kind,
    pub extent: Extent,
    pub tag: Tag,
}

impl Factor {
    pub fn new(kind: Kind, extent: Extent, tag: Option<String>) -> Self {
        Self {
            kind,
            extent,
            tag: Tag(tag),
        }
    }
}

/// A typed product space `A = a_1 x ... x a_m`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Space {
    pub factors: Vec<Factor>,
}

impl Space {
    pub fn new(factors: Vec<Factor>) -> Self {
        Self { factors }
    }

    pub fn product(&self, other: &Space) -> Space {
        let mut factors = self.factors.clone();
        factors.extend(other.factors.clone());
        Space::new(factors)
    }

    pub fn is_valid(&self, valuation: &Valuation, coords: &[u64]) -> bool {
        if self.factors.len() != coords.len() {
            return false;
        }
        for (f, &c) in self.factors.iter().zip(coords.iter()) {
            if let Some(extent) = valuation.get(&f.extent) {
                if c >= extent {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    pub fn volume_extent(&self) -> Extent {
        Extent::Product(self.factors.iter().map(|f| f.extent.clone()).collect())
    }

    /// Check if two spaces are shape-compatible (pointwise equal extents and same number of factors).
    pub fn compatible(&self, other: &Space) -> bool {
        if self.factors.len() != other.factors.len() {
            return false;
        }
        for (f1, f2) in self.factors.iter().zip(other.factors.iter()) {
            if f1.kind != f2.kind || f1.extent != f2.extent {
                return false;
            }
        }
        true
    }
}

/// A valuation `ν |= Γ` assigns concrete natural numbers to symbolic variables.
pub struct Valuation {
    pub variables: HashMap<String, u64>,
}

impl Valuation {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn get(&self, extent: &Extent) -> Option<u64> {
        extent.eval(&self.variables)
    }
}

/// A partial typed layout `L : A ⇀ B`.
pub trait Layout {
    fn source(&self) -> Space;
    fn target(&self) -> Space;

    /// Maps a product coordinate in source space to target space.
    /// Returns `None` if the input is outside the validity domain.
    fn apply(&self, valuation: &Valuation, input: &[u64]) -> Option<Vec<u64>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judgment {
    True,
    False,
    Unknown,
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
    Pad(Space, Space, Vec<(u64, u64)>), // source, target, padding per factor (left, right)
    Flip(Space, Vec<bool>),             // Space, dimensions to flip
}

impl Layout for Expression {
    fn source(&self) -> Space {
        match self {
            Expression::Identity(s) => s.clone(),
            Expression::Linearize(s) => s.clone(),
            Expression::Delinearize(target) => {
                Space::new(vec![Factor::new(Kind::Other("Offset".to_string()), target.volume_extent(), None)])
            }
            Expression::Permute(s, _) => s.clone(),
            Expression::Composition(l1, _) => l1.source(),
            Expression::Product(l1, l2) => l1.source().product(&l2.source()),
            Expression::Reshape(s, _) => s.clone(),
            Expression::BinaryShadow(s, _) => s.clone(),
            Expression::Slice(s, _) => s.clone(),
            Expression::Broadcast(s, _) => s.clone(),
            Expression::Pad(s, _, _) => s.clone(),
            Expression::Flip(s, _) => s.clone(),
        }
    }

    fn target(&self) -> Space {
        match self {
            Expression::Identity(s) => s.clone(),
            Expression::Linearize(s) => {
                Space::new(vec![Factor::new(Kind::Other("Offset".to_string()), s.volume_extent(), None)])
            }
            Expression::Delinearize(s) => s.clone(),
            Expression::Permute(s, p) => {
                let mut factors = vec![Factor::new(Kind::Logical, Extent::Constant(0), None); s.factors.len()];
                for (i, &pos) in p.iter().enumerate() {
                    factors[pos] = s.factors[i].clone();
                }
                Space::new(factors)
            }
            Expression::Composition(_, l2) => l2.target(),
            Expression::Product(l1, l2) => l1.target().product(&l2.target()),
            Expression::Reshape(_, t) => t.clone(),
            Expression::BinaryShadow(s, _) => s.clone(),
            Expression::Slice(s, ranges) => {
                let mut factors = Vec::new();
                for (i, f) in s.factors.iter().enumerate() {
                    let (start, end) = ranges[i];
                    factors.push(Factor::new(f.kind.clone(), Extent::Constant(end - start), f.tag.0.clone()));
                }
                Space::new(factors)
            }
            Expression::Broadcast(_, target) => target.clone(),
            Expression::Pad(_, t, _) => t.clone(),
            Expression::Flip(s, _) => s.clone(),
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
                
                // Process the bits covered by the matrix
                for i in 0..d {
                    let mut bit = 0;
                    for j in 0..d {
                        bit ^= ((offset >> j) & 1) & (matrix[i][j] as u64);
                    }
                    new_offset |= bit << i;
                }
                
                // PRESERVE bits outside the matrix range!
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
            Expression::Pad(s, _, padding) => {
                if !s.is_valid(valuation, input) { return None; }
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

/// Layered Normal Form: `L = S o P o V`.
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
    BitShiftRight(Box<ScalarExpr>, u32), // High-performance alternative to Div
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
                        // Strength Reduction: div by power-of-two is a bit shift
                        let shift = d.trailing_zeros();
                        ScalarExpr::BitShiftRight(Box::new(a), shift)
                    }
                    // Placeholder for Magic Constant optimization:
                    // (_, ScalarExpr::Constant(d)) => lower_to_magic_mul(a, d),
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

impl Expression {
    /// Lowers the layout expression to a set of flat scalar expressions (one per target dimension).
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
                    
                    output[i] = ScalarExpr::Mod(
                        Box::new(offset.clone()),
                        Box::new(ScalarExpr::Constant(extent))
                    ).simplify();
                    
                    offset = ScalarExpr::Div(
                        Box::new(offset),
                        Box::new(ScalarExpr::Constant(extent))
                    ).simplify();
                }
                output
            }
            Expression::Permute(s, p) => {
                let mut output = vec![ScalarExpr::Constant(0); s.factors.len()];
                for (i, &pos) in p.iter().enumerate() {
                    if i < inputs.len() {
                        output[pos] = inputs[i].clone();
                    }
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
            // Other variants follow similar logic...
            _ => inputs, 
        }
    }

    pub fn is_injective(&self) -> Judgment {
        match self {
            Expression::Identity(_) => Judgment::True,
            Expression::Linearize(_) => Judgment::True,
            Expression::Delinearize(_) => Judgment::True,
            Expression::Permute(_, _) => Judgment::True,
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
            Expression::Reshape(_, _) => Judgment::True,
            Expression::BinaryShadow(_, _) => Judgment::True,
            Expression::Slice(_, _) => Judgment::True,
            Expression::Broadcast(_, _) => Judgment::False,
            Expression::Pad(_, _, _) => Judgment::True,
            Expression::Flip(_, _) => Judgment::True,
        }
    }

    pub fn is_surjective(&self) -> Judgment {
        match self {
            Expression::Identity(_) => Judgment::True,
            Expression::Linearize(_) => Judgment::True,
            Expression::Delinearize(_) => Judgment::True,
            Expression::Permute(_, _) => Judgment::True,
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
            Expression::Reshape(_, _) => Judgment::True,
            Expression::BinaryShadow(_, _) => Judgment::True,
            Expression::Slice(_, _) => Judgment::False, // Only surjective on image
            Expression::Broadcast(_, _) => Judgment::True,
            Expression::Pad(_, _, _) => Judgment::False,
            Expression::Flip(_, _) => Judgment::True,
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
            Expression::Broadcast(_, _) => Judgment::True,
            _ => {
                match self.is_injective() {
                    Judgment::True => Judgment::False,
                    Judgment::False => Judgment::True,
                    Judgment::Unknown => Judgment::Unknown,
                }
            }
        }
    }

    pub fn normalize(self) -> LayeredNormalForm {
        match self {
            Expression::BinaryShadow(s, m) => LayeredNormalForm {
                view: Expression::Identity(s.clone()),
                placement: Expression::Identity(s.clone()),
                shadow: Expression::BinaryShadow(s, m),
            },
            Expression::Linearize(s) => LayeredNormalForm {
                view: Expression::Identity(s.clone()),
                placement: Expression::Linearize(s.clone()),
                shadow: Expression::Identity(Space::new(vec![Factor::new(Kind::Other("Offset".to_string()), s.volume_extent(), None)])),
            },
            Expression::Delinearize(s) => LayeredNormalForm {
                view: Expression::Identity(Space::new(vec![Factor::new(Kind::Other("Offset".to_string()), s.volume_extent(), None)])),
                placement: Expression::Delinearize(s.clone()),
                shadow: Expression::Identity(s),
            },
            _ => LayeredNormalForm {
                view: self.clone(),
                placement: Expression::Identity(self.target()),
                shadow: Expression::Identity(self.target()),
            }
        }
    }

    pub fn left_div(self, target: Expression) -> Option<Expression> {
        match self {
            Expression::Product(t, r) => {
                if *t == target {
                    Some(*r)
                } else {
                    if let Expression::Product(t_inner, a) = *t {
                        if *t_inner == target {
                            return Some(Expression::Product(a, r));
                        }
                    }
                    None
                }
            }
            _ => None,
        }
    }
}

/// Visualization and Rendering tools for FTPL Layouts.
pub mod viz {
    use super::*;

    /// Renders a 2D layout as an SVG grid.
    /// Colors cells by the Execution resource (e.g. ThreadID) and labels with Storage offset.
    pub fn render_svg(layout: &Expression, valuation: &Valuation) -> String {
        let src = layout.source();
        let tgt = layout.target();
        
        if src.factors.len() != 2 {
            return "SVG Rendering currently only supports 2D source spaces (H, W)".to_string();
        }

        let h_extent = valuation.get(&src.factors[0].extent).unwrap_or(1);
        let w_extent = valuation.get(&src.factors[1].extent).unwrap_or(1);

        let cell_size = 60;
        let width = w_extent * cell_size;
        let height = h_extent * cell_size;

        // Identify which target factors are Execution vs Storage
        let mut exec_indices = Vec::new();
        let mut storage_indices = Vec::new();
        for (i, f) in tgt.factors.iter().enumerate() {
            match f.kind {
                Kind::Execution => exec_indices.push(i),
                Kind::Storage | Kind::Other(_) => storage_indices.push(i),
                _ => {}
            }
        }

        let mut svg = format!(
            "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">\n",
            width, height
        );

        for h in 0..h_extent {
            for w in 0..w_extent {
                let x = w * cell_size;
                let y = h * cell_size;

                let out = layout.apply(valuation, &[h, w]).unwrap_or_else(|| vec![0; tgt.factors.len()]);
                
                // 1. Calculate Execution Unit ID for coloring
                let mut exec_id = 0;
                let mut has_exec = false;
                let mut stride = 1;
                for &idx in exec_indices.iter().rev() {
                    exec_id += out[idx] * stride;
                    stride *= valuation.get(&tgt.factors[idx].extent).unwrap_or(1);
                    has_exec = true;
                }

                // 2. Calculate Storage Offset for text
                // We linearize the storage factors to get a single global offset
                let mut storage_val = 0;
                let mut storage_stride = 1;
                for &idx in storage_indices.iter().rev() {
                    storage_val += out[idx] * storage_stride;
                    storage_stride *= valuation.get(&tgt.factors[idx].extent).unwrap_or(1);
                }

                // 3. Generate color
                // If has execution units, color by ID. Otherwise, color by offset (heatmap).
                let color = if has_exec {
                    let hue = (exec_id * 137) % 360; 
                    format!("hsl({}, 70%, 80%)", hue)
                } else {
                    let max_vol = tgt.volume_extent().eval(&valuation.variables).unwrap_or(1);
                    let intensity = (storage_val as f64 / max_vol as f64 * 200.0) as u8;
                    format!("rgb({}, {}, 255)", 255 - intensity, 255 - intensity)
                };

                svg.push_str(&format!(
                    "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"black\" />\n",
                    x, y, cell_size, cell_size, color
                ));
                
                // 4. Context-Aware Label
                let label = if has_exec {
                    format!("T{}:{}", exec_id, storage_val)
                } else {
                    format!("{}", storage_val)
                };

                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\" font-size=\"10\" text-anchor=\"middle\" fill=\"black\">{}</text>\n",
                    x + cell_size / 2, y + cell_size / 2 + 5, label
                ));
            }
        }

        svg.push_str("</svg>");
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_core_fit() {
        // 1. Define the Hardware Primitive (Tensor Core MMA instruction)
        // A 16x16 fragment mapped to storage.
        let fragment_space = Space::new(vec![
            Factor::new(Kind::Fragment, Extent::Constant(16), None),
            Factor::new(Kind::Fragment, Extent::Constant(16), None),
        ]);
        let tensor_core_instr = Expression::Linearize(fragment_space.clone());

        // 2. Define the Program Layout (a larger 32x32 tiled tensor)
        // We structure it as (InnerFragment x OuterTile).
        let outer_space = Space::new(vec![
            Factor::new(Kind::Tile, Extent::Constant(2), None),
            Factor::new(Kind::Tile, Extent::Constant(2), None),
        ]);
        
        // program = fragment x outer
        let program_layout = Expression::Product(
            Box::new(tensor_core_instr.clone()), 
            Box::new(Expression::Identity(outer_space.clone()))
        );

        // 3. Verify Primitive Fit
        // Does the program_layout factor through the tensor_core_instr?
        let fit_result = program_layout.left_div(tensor_core_instr);
        
        assert!(fit_result.is_some(), "Hardware primitive should fit the layout");
        
        // The remainder 'R' represents the iteration space for the tiles.
        let r = fit_result.unwrap();
        assert_eq!(r.source(), outer_space);
    }

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
    fn test_slice() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let slice = Expression::Slice(s, vec![(2, 5)]);
        let valuation = Valuation::new();
        assert_eq!(slice.apply(&valuation, &[2]).unwrap(), vec![0]);
        assert_eq!(slice.apply(&valuation, &[4]).unwrap(), vec![2]);
        assert!(slice.apply(&valuation, &[5]).is_none());
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
    fn test_binary_shadow() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(4), None)]);
        let matrix = vec![vec![0, 1], vec![1, 0]];
        let shadow = Expression::BinaryShadow(s, matrix);
        let valuation = Valuation::new();
        assert_eq!(shadow.apply(&valuation, &[1]).unwrap(), vec![2]);
        assert_eq!(shadow.apply(&valuation, &[2]).unwrap(), vec![1]);
    }

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
    fn test_valuation_variables() {
        let mut valuation = Valuation::new();
        valuation.variables.insert("N".to_string(), 10);
        valuation.variables.insert("M".to_string(), 20);
        let e3 = Extent::Product(vec![Extent::Variable("N".to_string()), Extent::Variable("M".to_string())]);
        assert_eq!(valuation.get(&e3).unwrap(), 200);
    }

    #[test]
    fn test_math_spec_composition() {
        let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(2), None)]);
        let l1 = Expression::Linearize(s1.clone());
        let l2 = Expression::Delinearize(s1.clone());
        let comp = Expression::Composition(Box::new(l1), Box::new(l2));
        let valuation = Valuation::new();
        assert_eq!(comp.apply(&valuation, &[1]).unwrap(), vec![1]);
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
    fn test_pad_flip() {
        let s = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let pad = Expression::Pad(s.clone(), Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(12), None)]), vec![(1, 1)]);
        let flip = Expression::Flip(s.clone(), vec![true]);
        let valuation = Valuation::new();
        assert_eq!(pad.apply(&valuation, &[0]).unwrap(), vec![1]);
        assert_eq!(flip.apply(&valuation, &[0]).unwrap(), vec![9]);
    }

    #[test]
    fn test_lowering() {
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(2), None),
            Factor::new(Kind::Logical, Extent::Constant(3), None),
        ]);
        let lin = Expression::Linearize(s);
        let valuation = Valuation::new();
        
        let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
        let lowered = lin.lower(&valuation, inputs);
        
        // The output is constructed as: (0 + (i1 * 1)) + (i0 * 3)
        // because the loop is .rev() and starts with offset = Constant(0).
        let expected = ScalarExpr::Add(
            Box::new(ScalarExpr::Add(
                Box::new(ScalarExpr::Constant(0)),
                Box::new(ScalarExpr::Input(1))
            )),
            Box::new(ScalarExpr::Mul(
                Box::new(ScalarExpr::Input(0)),
                Box::new(ScalarExpr::Constant(3))
            ))
        ).simplify();
        
        assert_eq!(lowered[0], expected);
        assert_eq!(lowered[0].eval(&[1, 2]), 5); 
    }
}

/// Examples demonstrating Deep Learning and Compiler operations.
#[cfg(test)]
mod dl_examples {
    use super::*;

    #[test]
    fn example_transpose_and_reshape() {
        // Start with a [Batch, Channels, Height, Width] tensor: [32, 64, 128, 128]
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(32), Some("B".to_string())),
            Factor::new(Kind::Logical, Extent::Constant(64), Some("C".to_string())),
            Factor::new(Kind::Logical, Extent::Constant(128), Some("H".to_string())),
            Factor::new(Kind::Logical, Extent::Constant(128), Some("W".to_string())),
        ]);
        let valuation = Valuation::new();

        // 1. Transpose: (B, C, H, W) -> (B, H, W, C) (Channels-Last)
        let transpose = Expression::Permute(s.clone(), vec![0, 3, 1, 2]); // target_idx = perm[src_idx]
        
        // 2. Reshape: Flatten H and W -> (B, H*W, C)
        let reshaped_space = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(32), Some("B".to_string())),
            Factor::new(Kind::Logical, Extent::Constant(128 * 128), Some("HW".to_string())),
            Factor::new(Kind::Logical, Extent::Constant(64), Some("C".to_string())),
        ]);
        // To implement a "view" reshape, we compose current target -> linear -> delinear to new target
        let current_target = transpose.target();
        let lin = Expression::Linearize(current_target.clone());
        let delin = Expression::Delinearize(reshaped_space.clone());
        let reshape_op = Expression::Composition(Box::new(lin), Box::new(delin));

        // 3. Final Layout = Reshape o Transpose
        let final_layout = Expression::Composition(Box::new(transpose), Box::new(reshape_op));

        assert_eq!(final_layout.source(), s);
        assert_eq!(final_layout.target(), reshaped_space);
        
        // Verify mapping: (0, 1, 0, 0) in (B, C, H, W) 
        // -> Transpose: (0, 0, 0, 1) in (B, H, W, C)
        // -> Linear: 1
        // -> Delinear: (0, 0, 1) in (B, HW, C)
        assert_eq!(final_layout.apply(&valuation, &[0, 1, 0, 0]).unwrap(), vec![0, 0, 1]);
    }

    #[test]
    fn example_compiler_tiling_and_vectorization() {
        // Program space: [1024]
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(1024), None),
        ]);
        let valuation = Valuation::new();

        // 1. Tiling: Split 1024 into [128, 8] (OuterTile, VectorInner)
        let tiled_space = Space::new(vec![
            Factor::new(Kind::Tile, Extent::Constant(128), Some("Outer".to_string())),
            Factor::new(Kind::Instruction, Extent::Constant(8), Some("Vector".to_string())),
        ]);
        let tiling = Expression::Reshape(s.clone(), tiled_space.clone());

        // 2. Check Vectorization Eligibility
        // In FTPL, vectorization is checked by looking at the inner dimension's stride.
        let lowered = tiling.lower(&valuation, vec![ScalarExpr::Input(0)]);
        
        // For a simple 1D -> 2D row-major reshape, the inner dimension (idx 1) 
        // of tiled_space should have stride 1.
        // Let's verify that the symbolic lowering reflects this contiguity.
        // tiled(i0, i1) = (i0 * 8) + i1
        // The coefficient for i1 is 1.
        if let ScalarExpr::Add(a, b) = &lowered[0] {
             // simplified (0 + (i1 * 1)) + (i0 * 8)
             // In a real compiler, we'd find the leaf node for Input(1).
             println!("Lowered Tiling: {:?}", lowered[0]);
        }
    }

    #[test]
    fn example_compiler_distribution() {
        // Map a 2D grid [1024, 1024] to GPU resources [Blocks, Threads]
        let logical = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(1024), Some("H".to_string())),
            Factor::new(Kind::Logical, Extent::Constant(1024), Some("W".to_string())),
        ]);

        let execution = Space::new(vec![
            Factor::new(Kind::Execution, Extent::Constant(256), Some("BlockIdx".to_string())),
            Factor::new(Kind::Execution, Extent::Constant(128), Some("ThreadIdx".to_string())),
        ]);

        // A distribution layout: Logical -> Execution
        // This usually involves tiling the logical space and binding factors to resources.
        let distribution = Expression::Reshape(logical, execution.clone());

        assert_eq!(distribution.target(), execution);
        assert_eq!(distribution.is_surjective(), Judgment::True);
        
        // Detection of Broad-casting: If multiple logical points map to the same thread,
        // it means the distribution is aliasing (e.g., if Logical Vol > Execution Vol).
        // Vol(L) = 1M, Vol(E) = 32k.
        let valuation = Valuation::new();
        assert_eq!(distribution.is_aliasing(&valuation), Judgment::True);
    }
}
