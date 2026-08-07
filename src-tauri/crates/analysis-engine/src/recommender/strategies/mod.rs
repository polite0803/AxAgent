//! 子策略导出

pub mod capital;
pub mod reversion;
pub mod serenity;
pub mod trend;
pub mod value;
pub mod watchlist;

pub use capital::CapitalStrategy;
pub use reversion::ReversionStrategy;
pub use serenity::SerenityStrategy;
pub use trend::TrendStrategy;
pub use value::ValueStrategy;
pub use watchlist::{emit_synthetic_picks, WatchlistStrategy};
