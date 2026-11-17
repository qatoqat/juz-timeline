pub mod data;
pub mod model;

use std::sync::LazyLock;

pub static MUSHAF: LazyLock<model::Mushaf> = LazyLock::new(model::Mushaf::build);
