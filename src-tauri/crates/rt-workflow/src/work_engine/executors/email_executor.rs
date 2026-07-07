// SPDX-License-Identifier: AGPL-3.0-only

//! Email executor — sends emails via SMTP using `lettre`.
//!
//! SMTP credentials are resolved from `credential_id` (via `CredentialManager`)
//! when available, falling back to inline `smtp_*` fields for backwards compatibility.

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct EmailExecutor;

impl EmailExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmailExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// SMTP configuration, resolved from credential or inline fields.
struct SmtpConfig {
    host: String,
    port: u16,
    user: String,
    pass: String,
    tls: bool,
}

fn resolve_smtp_config(
    ctx: &ExecutionState,
    credential_id: Option<&str>,
    host: Option<&str>,
    port: Option<u16>,
    user: Option<&str>,
    pass: Option<&str>,
) -> Result<SmtpConfig, NodeError> {
    if let Some(cid) = credential_id {
        let cm = ctx.credential_manager.as_ref().ok_or_else(|| {
            NodeError::exec_failed(
                "EMAIL_CREDENTIAL_UNAVAILABLE",
                "credential_manager not injected into ExecutionState",
            )
        })?;
        let sc = cm
            .get_smtp_config(cid)
            .map_err(|e| NodeError::exec_failed("EMAIL_CREDENTIAL_FAILED", e.to_string()))?;
        Ok(SmtpConfig {
            host: sc.host,
            port: sc.port,
            user: sc.user,
            pass: sc.pass,
            tls: sc.tls,
        })
    } else {
        let host = host
            .filter(|h| !h.is_empty())
            .ok_or_else(|| {
                NodeError::exec_failed(
                    "EMAIL_CONFIG_INCOMPLETE",
                    "smtp_host is required when credential_id is not set",
                )
            })?
            .to_string();
        let port = port.unwrap_or(587);
        let user = user.unwrap_or("").to_string();
        let pass = pass.unwrap_or("").to_string();
        // Default to TLS for port 587, plain for others
        let tls = port == 587;
        Ok(SmtpConfig {
            host,
            port,
            user,
            pass,
            tls,
        })
    }
}

#[async_trait]
impl NodeExecutorTrait for EmailExecutor {
    fn node_type(&self) -> &'static str {
        "email"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Email(n) = node else {
            return Err(NodeError::type_mismatch("email", self.node_type()));
        };
        let c = &n.config;

        let smtp = resolve_smtp_config(
            ctx,
            c.credential_id.as_deref(),
            c.smtp_host.as_deref(),
            c.smtp_port,
            c.smtp_user.as_deref(),
            c.smtp_pass.as_deref(),
        )?;

        tracing::info!(
            host = %smtp.host,
            port = smtp.port,
            to = ?c.to,
            subject = %c.subject,
            "EmailExecutor: sending email"
        );

        let from: Mailbox = smtp.user.parse().map_err(|e| {
            NodeError::exec_failed("EMAIL_PARSE_FAILED", format!("invalid from address: {e}"))
        })?;

        let mut msg_builder = Message::builder().from(from.clone()).subject(&c.subject);

        // Add recipients
        for addr in &c.to {
            let mbox: Mailbox = addr.parse().map_err(|e| {
                NodeError::exec_failed(
                    "EMAIL_PARSE_FAILED",
                    format!("invalid 'to' address '{addr}': {e}"),
                )
            })?;
            msg_builder = msg_builder.to(mbox);
        }

        let email_msg = msg_builder
            .header(lettre::message::header::ContentType::TEXT_PLAIN)
            .body(c.body.clone())
            .map_err(|e| NodeError::exec_failed("EMAIL_BUILD_FAILED", e.to_string()))?;

        let creds = Credentials::new(smtp.user.clone(), smtp.pass.clone());

        let mailer = if smtp.tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)
        } else {
            Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host))
        }
        .map_err(|e| NodeError::exec_failed("EMAIL_TRANSPORT_FAILED", e.to_string()))?
        .port(smtp.port)
        .credentials(creds)
        .build();

        match mailer.send(email_msg).await {
            Ok(_) => {
                tracing::info!("EmailExecutor: sent successfully");
            },
            Err(e) => {
                tracing::error!(%e, "EmailExecutor: send failed");
                return Err(NodeError::exec_failed("EMAIL_SEND_FAILED", e.to_string()));
            },
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "to": c.to,
                "subject": c.subject,
                "sent": true,
                "node_id": node.base_id(),
            }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
