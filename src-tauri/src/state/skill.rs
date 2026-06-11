//! Skill / plugin / sandbox domain state.
//!
//! Owns the skill-execution machinery: skill evolution, skill proposal
//! service, sandbox executor, dashboard / webhook registries, the
//! plugin manager, the sync engine, the ToT / planner scratch state, the
//! browser client, the various self-improvement engines
//! (text-grad, auto-tool-creator, intrinsic motivation, coevolution,
//! process-reward model, constitution), and the proactive service.

use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

#[allow(dead_code)]
pub struct SkillState {
    pub skill_evolution_engine:
        Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>>,
    pub skill_proposal_service:
        Arc<TokioRwLock<axagent_trajectory::SkillProposalService>>,
    pub skill_decomposer: Arc<tokio::sync::RwLock<axagent_trajectory::SkillDecomposer>>,
    #[cfg(not(target_os = "android"))]
    pub sandbox_executor: Arc<axagent_trajectory::SkillSandboxExecutor>,
    #[cfg(target_os = "android")]
    pub sandbox_executor: Arc<()>,
    pub dashboard_registry: Option<Arc<axagent_runtime::dashboard_registry::DashboardRegistry>>,
    pub webhook_subscription_manager:
        Option<Arc<axagent_runtime::webhook_subscription::WebhookSubscriptionManager>>,
    pub plugin_manager: Arc<tokio::sync::RwLock<axagent_plugins::PluginManager>>,
    pub sync_engine: Option<Arc<axagent_core::cloud_storage::SyncEngine>>,
    pub tot_sessions: Arc<tokio::sync::Mutex<std::collections::HashMap<String, crate::app_state::TotSession>>>,
    pub planner_sessions:
        Arc<tokio::sync::Mutex<std::collections::HashMap<String, crate::app_state::PlannerSession>>>,
    #[cfg(not(target_os = "android"))]
    pub browser_client: Arc<
        tokio::sync::Mutex<Option<axagent_core::browser_automation::PlaywrightClient>>,
    >,
    #[cfg(target_os = "android")]
    pub browser_client: Arc<tokio::sync::Mutex<Option<()>>>,
    pub text_grad_engine: Arc<tokio::sync::Mutex<axagent_trajectory::TextGradEngine>>,
    pub auto_tool_creator: Arc<tokio::sync::Mutex<axagent_trajectory::AutoToolCreator>>,
    pub intrinsic_motivation:
        Arc<tokio::sync::Mutex<axagent_trajectory::IntrinsicMotivationEngine>>,
    pub coevolution_env:
        Arc<tokio::sync::Mutex<axagent_trajectory::CoevolutionEnvironment>>,
    pub constitution: Arc<axagent_trajectory::ImmutableConstitution>,
    pub process_reward_model:
        Arc<tokio::sync::Mutex<axagent_trajectory::ProcessRewardModel>>,
    pub proactive_service: Arc<tokio::sync::RwLock<crate::commands::proactive::ProactiveService>>,
}

#[allow(dead_code)]
impl SkillState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        skill_evolution_engine: Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>>,
        skill_proposal_service: Arc<TokioRwLock<axagent_trajectory::SkillProposalService>>,
        skill_decomposer: Arc<tokio::sync::RwLock<axagent_trajectory::SkillDecomposer>>,
        sandbox_executor: SandboxExecutorField,
        dashboard_registry: Option<
            Arc<axagent_runtime::dashboard_registry::DashboardRegistry>,
        >,
        webhook_subscription_manager: Option<
            Arc<axagent_runtime::webhook_subscription::WebhookSubscriptionManager>,
        >,
        plugin_manager: Arc<tokio::sync::RwLock<axagent_plugins::PluginManager>>,
        sync_engine: Option<Arc<axagent_core::cloud_storage::SyncEngine>>,
        tot_sessions: Arc<
            tokio::sync::Mutex<
                std::collections::HashMap<String, crate::app_state::TotSession>,
            >,
        >,
        planner_sessions: Arc<
            tokio::sync::Mutex<
                std::collections::HashMap<String, crate::app_state::PlannerSession>,
            >,
        >,
        browser_client: BrowserClientField,
        text_grad_engine: Arc<tokio::sync::Mutex<axagent_trajectory::TextGradEngine>>,
        auto_tool_creator: Arc<tokio::sync::Mutex<axagent_trajectory::AutoToolCreator>>,
        intrinsic_motivation: Arc<
            tokio::sync::Mutex<axagent_trajectory::IntrinsicMotivationEngine>,
        >,
        coevolution_env: Arc<tokio::sync::Mutex<axagent_trajectory::CoevolutionEnvironment>>,
        constitution: Arc<axagent_trajectory::ImmutableConstitution>,
        process_reward_model: Arc<tokio::sync::Mutex<axagent_trajectory::ProcessRewardModel>>,
        proactive_service: Arc<
            tokio::sync::RwLock<crate::commands::proactive::ProactiveService>,
        >,
    ) -> Self {
        Self {
            skill_evolution_engine,
            skill_proposal_service,
            skill_decomposer,
            #[cfg(not(target_os = "android"))]
            sandbox_executor: match sandbox_executor {
                SandboxExecutorField::Real(v) => v,
                SandboxExecutorField::Dummy => {
                    panic!("SandboxExecutorField mismatch (dummy provided on non-android)")
                }
            },
            #[cfg(target_os = "android")]
            sandbox_executor: match sandbox_executor {
                SandboxExecutorField::Dummy => Arc::new(()),
                SandboxExecutorField::Real(_) => {
                    panic!("SandboxExecutorField mismatch (real provided on android)")
                }
            },
            dashboard_registry,
            webhook_subscription_manager,
            plugin_manager,
            sync_engine,
            tot_sessions,
            planner_sessions,
            #[cfg(not(target_os = "android"))]
            browser_client: match browser_client {
                BrowserClientField::Real(v) => v,
                BrowserClientField::Dummy => {
                    panic!("BrowserClientField mismatch (dummy provided on non-android)")
                }
            },
            #[cfg(target_os = "android")]
            browser_client: match browser_client {
                BrowserClientField::Dummy => Arc::new(tokio::sync::Mutex::new(None)),
                BrowserClientField::Real(_) => {
                    panic!("BrowserClientField mismatch (real provided on android)")
                }
            },
            text_grad_engine,
            auto_tool_creator,
            intrinsic_motivation,
            coevolution_env,
            constitution,
            process_reward_model,
            proactive_service,
        }
    }
}

/// Cross-platform wrapper for the sandbox executor type, which differs
/// between desktop and Android builds.  Callers construct the right
/// variant from `create_app_state` (where the `cfg` arms are already
/// handled) and pass it into [`SkillState::new`].
#[allow(dead_code)]
pub enum SandboxExecutorField {
    Real(Arc<axagent_trajectory::SkillSandboxExecutor>),
    Dummy,
}

/// Cross-platform wrapper for the browser-client type, which differs
/// between desktop and Android builds.
#[allow(dead_code)]
pub enum BrowserClientField {
    Real(Arc<tokio::sync::Mutex<Option<axagent_core::browser_automation::PlaywrightClient>>>),
    Dummy,
}
