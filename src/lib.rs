pub mod core;
pub mod layout;
pub mod analysis;
pub mod viz;

// Re-export core types for convenience
pub use crate::core::{Kind, Extent, Tag, Factor, Space, Valuation};
pub use crate::layout::{Expression, Layout, ScalarExpr};
pub use crate::analysis::{Judgment, LayeredNormalForm};
