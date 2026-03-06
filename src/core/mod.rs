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
    Offset,
    Other(String),
}

/// A symbolic natural-number value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Extent {
    Constant(u64),
    Variable(String),
    Product(Vec<Extent>),
    Quotient(Box<Extent>, Box<Extent>),
    Remainder(Box<Extent>, Box<Extent>),
}

impl Extent {
    pub fn try_eval(&self, variables: &HashMap<String, u64>) -> Option<u64> {
        match self {
            Extent::Constant(v) => Some(*v),
            Extent::Variable(name) => variables.get(name).copied(),
            Extent::Product(parts) => {
                let mut res: u64 = 1;
                for p in parts {
                    res = res.checked_mul(p.try_eval(variables)?)?;
                }
                Some(res)
            }
            Extent::Quotient(a, b) => {
                let va = a.try_eval(variables)?;
                let vb = b.try_eval(variables)?;
                if vb == 0 { None } else { Some(va / vb) }
            }
            Extent::Remainder(a, b) => {
                let va = a.try_eval(variables)?;
                let vb = b.try_eval(variables)?;
                if vb == 0 { None } else { Some(va % vb) }
            }
        }
    }

    pub fn simplify(self) -> Self {
        match self {
            Extent::Product(mut parts) => {
                if parts.is_empty() {
                    return Extent::Constant(1);
                }
                parts = parts.into_iter().map(|p| p.simplify()).collect();
                let mut const_val = 1;
                let mut new_parts = Vec::new();
                for p in parts {
                    if let Extent::Constant(v) = p {
                        const_val *= v;
                    } else {
                        new_parts.push(p);
                    }
                }
                if const_val != 1 || new_parts.is_empty() {
                    new_parts.insert(0, Extent::Constant(const_val));
                }
                if new_parts.len() == 1 {
                    new_parts.pop().unwrap()
                } else {
                    Extent::Product(new_parts)
                }
            }
            _ => self,
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

    pub fn rank(&self) -> usize {
        self.factors.len()
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
            if let Some(extent) = f.extent.try_eval(&valuation.variables) {
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
        Extent::Product(self.factors.iter().map(|f| f.extent.clone()).collect()).simplify()
    }

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

    pub fn permute(&self, p: &[usize]) -> Space {
        let mut factors =
            vec![Factor::new(Kind::Logical, Extent::Constant(0), None); self.factors.len()];
        for (i, &pos) in p.iter().enumerate() {
            factors[pos] = self.factors[i].clone();
        }
        Space::new(factors)
    }

    pub fn linearize(&self, coords: &[u64], valuation: &Valuation) -> Option<u64> {
        if !self.is_valid(valuation, coords) {
            return None;
        }
        let mut offset = 0;
        let mut stride = 1;
        for (f, &c) in self.factors.iter().rev().zip(coords.iter().rev()) {
            offset += c * stride;
            stride *= f.extent.try_eval(&valuation.variables)?;
        }
        Some(offset)
    }

    pub fn delinearize(&self, mut offset: u64, valuation: &Valuation) -> Option<Vec<u64>> {
        let mut output = vec![0; self.factors.len()];
        for (i, f) in self.factors.iter().enumerate().rev() {
            let extent = f.extent.try_eval(&valuation.variables)?;
            output[i] = offset % extent;
            offset /= extent;
        }
        if offset > 0 {
            return None;
        }
        Some(output)
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

    pub fn get_extent(&self, extent: &Extent) -> Option<u64> {
        extent.try_eval(&self.variables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valuation_variables() {
        let mut valuation = Valuation::new();
        valuation.variables.insert("N".to_string(), 10);
        valuation.variables.insert("M".to_string(), 20);
        let e3 = Extent::Product(vec![
            Extent::Variable("N".to_string()),
            Extent::Variable("M".to_string()),
        ]);
        assert_eq!(valuation.get_extent(&e3).unwrap(), 200);
    }

    #[test]
    fn test_space_product_and_volume() {
        let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(2), None)]);
        let s2 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(3), None)]);
        let s3 = s1.product(&s2);

        assert_eq!(s3.factors.len(), 2);
        let val = Valuation::new();
        assert_eq!(val.get_extent(&s3.volume_extent()).unwrap(), 6);
    }

    #[test]
    fn test_space_validity() {
        let s = Space::new(vec![
            Factor::new(Kind::Logical, Extent::Constant(2), None),
            Factor::new(Kind::Logical, Extent::Constant(3), None),
        ]);
        let val = Valuation::new();

        assert!(s.is_valid(&val, &[0, 0]));
        assert!(s.is_valid(&val, &[1, 2]));
        assert!(!s.is_valid(&val, &[2, 0]));
        assert!(!s.is_valid(&val, &[0, 3]));
        assert!(!s.is_valid(&val, &[0]));
    }

    #[test]
    fn test_space_compatibility() {
        let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let s2 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let s3 = Space::new(vec![Factor::new(Kind::Storage, Extent::Constant(10), None)]);

        assert!(s1.compatible(&s2));
        assert!(!s1.compatible(&s3));
    }
}
