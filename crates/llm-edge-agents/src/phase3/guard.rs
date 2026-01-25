//! Execution Role Guard
//!
//! Enforces Phase 3 Layer 1 execution role constraints:
//! - Agents MUST: Coordinate, Route, Optimize, Escalate
//! - Agents MUST NOT: Make final decisions, Emit executive conclusions
//!
//! HARD FAIL on execution guard violations.

use std::collections::HashSet;
use tracing::{error, warn};

/// Allowed agent roles in Phase 3 Layer 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Coordinate between components/agents
    Coordinate,
    /// Route requests/traffic
    Route,
    /// Optimize performance/resources
    Optimize,
    /// Escalate issues/decisions
    Escalate,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Coordinate => write!(f, "coordinate"),
            AgentRole::Route => write!(f, "route"),
            AgentRole::Optimize => write!(f, "optimize"),
            AgentRole::Escalate => write!(f, "escalate"),
        }
    }
}

/// Forbidden actions in Phase 3 Layer 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForbiddenAction {
    /// Making final/executive decisions
    FinalDecision,
    /// Emitting executive conclusions
    ExecutiveConclusion,
    /// Directly modifying state without coordination
    DirectStateModification,
    /// Bypassing escalation for critical actions
    EscalationBypass,
}

impl std::fmt::Display for ForbiddenAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForbiddenAction::FinalDecision => write!(f, "final_decision"),
            ForbiddenAction::ExecutiveConclusion => write!(f, "executive_conclusion"),
            ForbiddenAction::DirectStateModification => write!(f, "direct_state_modification"),
            ForbiddenAction::EscalationBypass => write!(f, "escalation_bypass"),
        }
    }
}

/// Role violation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum RoleViolation {
    #[error("Forbidden action attempted: {action} by agent {agent_id}")]
    ForbiddenActionAttempted {
        agent_id: String,
        action: String,
        context: String,
    },

    #[error("Role constraint violated: agent {agent_id} attempted {attempted_role} but only allowed {allowed_roles:?}")]
    RoleConstraintViolated {
        agent_id: String,
        attempted_role: String,
        allowed_roles: Vec<String>,
    },

    #[error("Missing required escalation for action: {action}")]
    MissingEscalation {
        action: String,
        required_escalation_path: String,
    },
}

/// Execution role guard for Phase 3 Layer 1 agents
pub struct ExecutionRoleGuard {
    agent_id: String,
    allowed_roles: HashSet<AgentRole>,
    strict_mode: bool,
}

impl ExecutionRoleGuard {
    /// Create a new execution role guard with default Phase 3 Layer 1 roles
    pub fn new(agent_id: impl Into<String>) -> Self {
        let mut allowed_roles = HashSet::new();
        allowed_roles.insert(AgentRole::Coordinate);
        allowed_roles.insert(AgentRole::Route);
        allowed_roles.insert(AgentRole::Optimize);
        allowed_roles.insert(AgentRole::Escalate);

        Self {
            agent_id: agent_id.into(),
            allowed_roles,
            strict_mode: true,
        }
    }

    /// Create with custom allowed roles
    pub fn with_roles(agent_id: impl Into<String>, roles: Vec<AgentRole>) -> Self {
        Self {
            agent_id: agent_id.into(),
            allowed_roles: roles.into_iter().collect(),
            strict_mode: true,
        }
    }

    /// Enable or disable strict mode (strict = HARD FAIL on violations)
    pub fn strict_mode(mut self, enabled: bool) -> Self {
        self.strict_mode = enabled;
        self
    }

    /// Check if a role is allowed
    pub fn is_role_allowed(&self, role: AgentRole) -> bool {
        self.allowed_roles.contains(&role)
    }

    /// Validate that an action is allowed - HARD FAIL in strict mode
    pub fn validate_action(&self, action: &ActionContext) -> Result<(), RoleViolation> {
        // Check for forbidden actions
        if let Some(forbidden) = self.detect_forbidden_action(action) {
            let violation = RoleViolation::ForbiddenActionAttempted {
                agent_id: self.agent_id.clone(),
                action: forbidden.to_string(),
                context: action.description.clone(),
            };

            if self.strict_mode {
                error!(
                    agent_id = %self.agent_id,
                    action = %forbidden,
                    context = %action.description,
                    "HARD FAIL: Execution guard violation - forbidden action"
                );
                return Err(violation);
            } else {
                warn!(
                    agent_id = %self.agent_id,
                    action = %forbidden,
                    "Execution guard warning - forbidden action (non-strict mode)"
                );
            }
        }

        // Check role constraints
        if !self.allowed_roles.contains(&action.role) {
            let violation = RoleViolation::RoleConstraintViolated {
                agent_id: self.agent_id.clone(),
                attempted_role: action.role.to_string(),
                allowed_roles: self.allowed_roles.iter().map(|r| r.to_string()).collect(),
            };

            if self.strict_mode {
                error!(
                    agent_id = %self.agent_id,
                    attempted_role = %action.role,
                    "HARD FAIL: Execution guard violation - role constraint"
                );
                return Err(violation);
            } else {
                warn!(
                    agent_id = %self.agent_id,
                    attempted_role = %action.role,
                    "Execution guard warning - role constraint (non-strict mode)"
                );
            }
        }

        // Check escalation requirements for critical actions
        if action.is_critical && !action.has_escalation {
            let violation = RoleViolation::MissingEscalation {
                action: action.description.clone(),
                required_escalation_path: "layer2_coordinator".to_string(),
            };

            if self.strict_mode {
                error!(
                    agent_id = %self.agent_id,
                    action = %action.description,
                    "HARD FAIL: Execution guard violation - missing escalation for critical action"
                );
                return Err(violation);
            }
        }

        Ok(())
    }

    /// Detect if an action contains forbidden operations
    fn detect_forbidden_action(&self, action: &ActionContext) -> Option<ForbiddenAction> {
        // Check for final decision indicators
        let final_decision_keywords = [
            "final_decision",
            "executive_decision",
            "commit_decision",
            "approve_final",
            "reject_final",
        ];

        let description_lower = action.description.to_lowercase();

        for keyword in final_decision_keywords {
            if description_lower.contains(keyword) {
                return Some(ForbiddenAction::FinalDecision);
            }
        }

        // Check for executive conclusion indicators
        let conclusion_keywords = [
            "executive_conclusion",
            "final_conclusion",
            "concluded",
            "definitive_answer",
        ];

        for keyword in conclusion_keywords {
            if description_lower.contains(keyword) {
                return Some(ForbiddenAction::ExecutiveConclusion);
            }
        }

        // Check action type
        if action.is_state_modification && !action.is_coordinated {
            return Some(ForbiddenAction::DirectStateModification);
        }

        if action.bypasses_escalation && action.is_critical {
            return Some(ForbiddenAction::EscalationBypass);
        }

        None
    }

    /// Create a coordination action context
    pub fn coordination_action(description: impl Into<String>) -> ActionContext {
        ActionContext {
            role: AgentRole::Coordinate,
            description: description.into(),
            is_critical: false,
            has_escalation: false,
            is_state_modification: false,
            is_coordinated: true,
            bypasses_escalation: false,
        }
    }

    /// Create a routing action context
    pub fn routing_action(description: impl Into<String>) -> ActionContext {
        ActionContext {
            role: AgentRole::Route,
            description: description.into(),
            is_critical: false,
            has_escalation: false,
            is_state_modification: false,
            is_coordinated: true,
            bypasses_escalation: false,
        }
    }

    /// Create an optimization action context
    pub fn optimization_action(description: impl Into<String>) -> ActionContext {
        ActionContext {
            role: AgentRole::Optimize,
            description: description.into(),
            is_critical: false,
            has_escalation: false,
            is_state_modification: false,
            is_coordinated: true,
            bypasses_escalation: false,
        }
    }

    /// Create an escalation action context
    pub fn escalation_action(description: impl Into<String>, target: impl Into<String>) -> ActionContext {
        ActionContext {
            role: AgentRole::Escalate,
            description: format!("{} -> {}", description.into(), target.into()),
            is_critical: true,
            has_escalation: true,
            is_state_modification: false,
            is_coordinated: true,
            bypasses_escalation: false,
        }
    }
}

/// Context for an action being validated
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// The role being exercised
    pub role: AgentRole,
    /// Description of the action
    pub description: String,
    /// Whether this is a critical action requiring escalation
    pub is_critical: bool,
    /// Whether escalation has been performed
    pub has_escalation: bool,
    /// Whether this modifies state
    pub is_state_modification: bool,
    /// Whether this is a coordinated action
    pub is_coordinated: bool,
    /// Whether this bypasses escalation requirements
    pub bypasses_escalation: bool,
}

impl ActionContext {
    /// Mark action as critical
    pub fn critical(mut self) -> Self {
        self.is_critical = true;
        self
    }

    /// Mark action as having escalation
    pub fn with_escalation(mut self) -> Self {
        self.has_escalation = true;
        self
    }

    /// Mark action as state modification
    pub fn state_modification(mut self) -> Self {
        self.is_state_modification = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_roles() {
        let guard = ExecutionRoleGuard::new("test_agent");

        assert!(guard.is_role_allowed(AgentRole::Coordinate));
        assert!(guard.is_role_allowed(AgentRole::Route));
        assert!(guard.is_role_allowed(AgentRole::Optimize));
        assert!(guard.is_role_allowed(AgentRole::Escalate));
    }

    #[test]
    fn test_coordination_action_allowed() {
        let guard = ExecutionRoleGuard::new("test_agent");
        let action = ExecutionRoleGuard::coordination_action("coordinate failover");

        assert!(guard.validate_action(&action).is_ok());
    }

    #[test]
    fn test_forbidden_final_decision() {
        let guard = ExecutionRoleGuard::new("test_agent");
        let action = ActionContext {
            role: AgentRole::Coordinate,
            description: "final_decision: approve request".to_string(),
            is_critical: false,
            has_escalation: false,
            is_state_modification: false,
            is_coordinated: true,
            bypasses_escalation: false,
        };

        assert!(matches!(
            guard.validate_action(&action),
            Err(RoleViolation::ForbiddenActionAttempted { .. })
        ));
    }

    #[test]
    fn test_critical_action_requires_escalation() {
        let guard = ExecutionRoleGuard::new("test_agent");
        let action = ActionContext {
            role: AgentRole::Coordinate,
            description: "critical operation".to_string(),
            is_critical: true,
            has_escalation: false, // Missing escalation
            is_state_modification: false,
            is_coordinated: true,
            bypasses_escalation: false,
        };

        assert!(matches!(
            guard.validate_action(&action),
            Err(RoleViolation::MissingEscalation { .. })
        ));
    }

    #[test]
    fn test_escalation_action_allowed() {
        let guard = ExecutionRoleGuard::new("test_agent");
        let action = ExecutionRoleGuard::escalation_action(
            "escalate circuit breaker trip",
            "layer2_coordinator"
        );

        assert!(guard.validate_action(&action).is_ok());
    }
}
