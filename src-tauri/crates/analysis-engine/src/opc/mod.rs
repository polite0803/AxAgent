// OPC (One-Person Company) 模块，融合自原 opc-types 和 opc-dao

pub mod analysis;
pub mod analytics;
pub mod audit_log;
pub mod automation;
pub mod customer;
pub mod data_service;
pub mod error;
pub mod finance;
pub mod industry;
pub mod invoice;
pub mod learning;
pub mod project;
pub mod rules;
pub mod site;
pub mod vendors;
pub mod workflow;

pub use analysis::*;
pub use analytics::*;
pub use audit_log::*;
pub use automation::*;
pub use customer::*;
pub use data_service::*;
pub use error::*;
pub use finance::*;
pub use industry::*;
pub use invoice::*;
pub use learning::*;
pub use project::*;
pub use rules::*;
pub use site::*;
pub use vendors::*;
pub use workflow::*;
