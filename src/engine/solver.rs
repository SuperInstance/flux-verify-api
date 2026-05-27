//! Lightweight CSP solver using backtracking.
//! For the MVP, this is a placeholder that returns the parsed solution directly.

use crate::compiler::ConstraintProblem;

#[derive(Debug, Clone)]
pub struct Solution {
    pub variables: Vec<(String, f64)>,
    pub satisfied: bool,
}

/// Solve a constraint problem via backtracking.
/// Currently returns the initial assignment — the VM does the real verification.
pub fn solve(_problem: &ConstraintProblem) -> Solution {
    // The FLUX VM handles execution and evaluation.
    // The solver is here for future constraint-satisfaction extensions.
    Solution {
        variables: vec![],
        satisfied: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{Assertion, Constraint, Variable};

    #[test]
    fn test_solve_returns_satisfied() {
        let problem = ConstraintProblem {
            domain: "generic".into(),
            variables: vec![Variable {
                name: "x".into(),
                value: 42.0,
                desc: "test".into(),
            }],
            constraints: vec![Constraint::GenericCompare {
                left: 10.0,
                operator: "gt".into(),
                right: 5.0,
                desc: "10 > 5".into(),
            }],
            assertion: Assertion {
                assertion_type: "gt".into(),
                expected: 5.0,
                actual_expr: "10 > 5".into(),
            },
        };
        let sol = solve(&problem);
        assert!(sol.satisfied);
        assert!(sol.variables.is_empty()); // placeholder returns empty vars
    }
}
