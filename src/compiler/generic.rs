use super::{parser, ConstraintProblem};

/// Generic constraint parser.
/// Handles comparison operators, bounds, and simple numeric constraints.
pub fn parse(claim: &str) -> Result<ConstraintProblem, String> {
    let lower = claim.to_lowercase();
    let original = claim.to_string();

    // Try to parse as a comparison: "X is greater than Y", "X > Y", "X is at least Y"
    if let Some((left, op, right, _desc)) = parser::extract_comparison(&lower) {
        return Ok(ConstraintProblem {
            domain: "generic".into(),
            variables: vec![
                super::Variable {
                    name: "left".into(),
                    value: left,
                    desc: "left operand".into(),
                },
                super::Variable {
                    name: "right".into(),
                    value: right,
                    desc: "right operand".into(),
                },
            ],
            constraints: vec![super::Constraint::GenericCompare {
                left,
                operator: op.clone(),
                right,
                desc: original.clone(),
            }],
            assertion: super::Assertion {
                assertion_type: op,
                expected: right,
                actual_expr: original,
            },
        });
    }

    // Try to parse as a range check: "X is between Y and Z", "X is within [Y, Z]"
    if let Some((value, min, max, _desc)) = parser::extract_range_check(&lower) {
        return Ok(ConstraintProblem {
            domain: "generic".into(),
            variables: vec![
                super::Variable {
                    name: "value".into(),
                    value,
                    desc: "value".into(),
                },
                super::Variable {
                    name: "min".into(),
                    value: min,
                    desc: "minimum bound".into(),
                },
                super::Variable {
                    name: "max".into(),
                    value: max,
                    desc: "maximum bound".into(),
                },
            ],
            constraints: vec![super::Constraint::GenericRangeCheck {
                value,
                min,
                max,
                desc: original.clone(),
            }],
            assertion: super::Assertion {
                assertion_type: "in_range".into(),
                expected: 0.0,
                actual_expr: original,
            },
        });
    }

    // Try to parse as a simple bound: "X is within Y of Z"
    if let Some((value, min, max, _desc)) = parser::extract_bound(&lower) {
        return Ok(ConstraintProblem {
            domain: "generic".into(),
            variables: vec![
                super::Variable {
                    name: "value".into(),
                    value,
                    desc: "value".into(),
                },
                super::Variable {
                    name: "min".into(),
                    value: min,
                    desc: "minimum".into(),
                },
                super::Variable {
                    name: "max".into(),
                    value: max,
                    desc: "maximum".into(),
                },
            ],
            constraints: vec![super::Constraint::GenericBound {
                value,
                min,
                max,
                desc: original.clone(),
            }],
            assertion: super::Assertion {
                assertion_type: "in_range".into(),
                expected: 0.0,
                actual_expr: original,
            },
        });
    }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comparison_gt() {
        let problem = parse("10 is greater than 5").unwrap();
        assert_eq!(problem.domain, "generic");
        assert_eq!(problem.variables.len(), 2);
    }

    #[test]
    fn test_parse_comparison_lt() {
        let problem = parse("3 is less than 10").unwrap();
        assert_eq!(problem.domain, "generic");
        let has_compare = problem.constraints.iter().any(|c| matches!(c, crate::compiler::Constraint::GenericCompare { .. }));
        assert!(has_compare);
    }

    #[test]
    fn test_parse_direct_operator() {
        let problem = parse("100 > 50").unwrap();
        assert_eq!(problem.domain, "generic");
    }

    #[test]
    fn test_parse_range_check() {
        let problem = parse("50 is between 20 and 80").unwrap();
        assert_eq!(problem.domain, "generic");
        let has_range = problem.constraints.iter().any(|c| matches!(c, crate::compiler::Constraint::GenericRangeCheck { .. }));
        assert!(has_range);
    }

    #[test]
    fn test_parse_bound() {
        let problem = parse("52 is within 3 of 50").unwrap();
        assert_eq!(problem.domain, "generic");
        let has_bound = problem.constraints.iter().any(|c| matches!(c, crate::compiler::Constraint::GenericBound { .. }));
        assert!(has_bound);
    }

    #[test]
    fn test_parse_unrecognized() {
        let result = parse("the quick brown fox");
        assert!(result.is_err());
    }
}

    Err("Could not parse claim as a generic constraint. Try: 'X is greater than Y', 'X is between Y and Z', or 'X > Y'".into())
}
