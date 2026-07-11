//! Tool creation domain state.
//!
//! Owns the auto-tool-creator engine that generates new tools
//! from learned patterns in trajectory data.

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ToolState {
    pub auto_tool_creator: Arc<Mutex<axagent_trajectory::AutoToolCreator>>,
}

impl ToolState {
    pub fn new(auto_tool_creator: Arc<Mutex<axagent_trajectory::AutoToolCreator>>) -> Self {
        Self { auto_tool_creator }
    }
}
