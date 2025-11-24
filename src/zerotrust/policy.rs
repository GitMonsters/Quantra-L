use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::zerotrust::{AccessDecision, identity::Identity};

/// Policy Engine evaluates access requests
pub struct PolicyEngine {
    policies: Vec<Policy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub rules: Vec<Rule>,
    pub action: PolicyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub attribute: String,
    pub operator: Operator,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    Allow,
    Deny,
    RequireMFA,
    RequireVMIsolation,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let default_policies = vec![
            Policy {
                name: "critical_resources_require_isolation".to_string(),
                rules: vec![Rule {
                    attribute: "resource_type".to_string(),
                    operator: Operator::Equals,
                    value: "critical".to_string(),
                }],
                action: PolicyAction::RequireVMIsolation,
            },
            Policy {
                name: "untrusted_users_denied".to_string(),
                rules: vec![Rule {
                    attribute: "trust_score".to_string(),
                    operator: Operator::LessThan,
                    value: "20".to_string(),
                }],
                action: PolicyAction::Deny,
            },
        ];

        Self {
            policies: default_policies,
        }
    }

    pub async fn evaluate(
        &self,
        identity: &Identity,
        requested_resources: &[String],
    ) -> Result<AccessDecision> {
        for policy in &self.policies {
            if self.matches_policy(identity, requested_resources, policy) {
                match &policy.action {
                    PolicyAction::Allow => return Ok(AccessDecision::Allow),
                    PolicyAction::Deny => {
                        return Ok(AccessDecision::Deny(format!(
                            "Denied by policy: {}",
                            policy.name
                        )))
                    }
                    PolicyAction::RequireMFA => {
                        return Ok(AccessDecision::AllowWithConditions(vec![
                            "MFA required".to_string(),
                        ]))
                    }
                    PolicyAction::RequireVMIsolation => {
                        return Ok(AccessDecision::AllowWithConditions(vec![
                            "VM isolation required".to_string(),
                        ]))
                    }
                }
            }
        }

        Ok(AccessDecision::Allow)
    }

    fn matches_policy(
        &self,
        _identity: &Identity,
        requested_resources: &[String],
        policy: &Policy,
    ) -> bool {
        requested_resources
            .iter()
            .any(|r| r.starts_with("critical/"))
    }
}
