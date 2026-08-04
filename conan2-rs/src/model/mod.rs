mod model;
mod visit;

pub use model::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Recipe Not Found {0}")]
    RecipeNotFound(String),
    #[error("Invalid Dependency {0}")]
    InvalidDependency(String),
}
