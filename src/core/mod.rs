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
        let mut factors = vec![Factor::new(Kind::Logical, Extent::Constant(0), None); self.factors.len()];
        for (i, &pos) in p.iter().enumerate() {
            factors[pos] = self.factors[i].clone();
        }
        Space::new(factors)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valuation_variables() {
        let mut valuation = Valuation::new();
        valuation.variables.insert("N".to_string(), 10);
        valuation.variables.insert("M".to_string(), 20);
        let e3 = Extent::Product(vec![Extent::Variable("N".to_string()), Extent::Variable("M".to_string())]);
        assert_eq!(valuation.get(&e3).unwrap(), 200);
    }

    #[test]
    fn test_space_product_and_volume() {
        let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(2), None)]);
        let s2 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(3), None)]);
        let s3 = s1.product(&s2);
        
        assert_eq!(s3.factors.len(), 2);
        let mut val = Valuation::new();
        assert_eq!(val.get(&s3.volume_extent()).unwrap(), 6);
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
        assert!(!s.is_valid(&val, &[0])); // Wrong rank
    }

    #[test]
    fn test_space_compatibility() {
        let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let s2 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
        let s3 = Space::new(vec![Factor::new(Kind::Storage, Extent::Constant(10), None)]);
        
        assert!(s1.compatible(&s2));
        assert!(!s1.compatible(&s3)); // Different kinds
    }
}
