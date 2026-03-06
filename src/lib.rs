pub mod analysis;
pub mod core;
pub mod layout;
pub mod viz;

// Re-export core types for convenience
pub use crate::analysis::{Judgment, LayeredNormalForm};
pub use crate::core::{Extent, Factor, Kind, Space, Tag, Valuation};
pub use crate::layout::{Expression, Layout, ScalarExpr};
