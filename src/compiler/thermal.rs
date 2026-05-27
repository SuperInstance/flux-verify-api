use super::{parser, ConstraintProblem};

/// Thermal domain parser.
/// Extracts temperature bounds and safe ranges from natural language.
pub fn parse(claim: &str) -> Result<ConstraintProblem, String> {
    let lower = claim.to_lowercase();

    // Extract the temperature to check
    let temp_c = parser::extract_number_near(&lower, "temperature")
        .or_else(|| parser::extract_number_near(&lower, "temp"))
        .or_else(|| parser::extract_number_before(&lower, "°c"))
        .or_else(|| parser::extract_number_before(&lower, "degrees"))
        .ok_or("Could not extract temperature from claim")?;

    // Extract safe range bounds (e.g., "safe range of X to Y", "range of X to Y")
    let (min_safe, max_safe) = parser::extract_range(&lower)
        .ok_or("Could not extract safe temperature range from claim")?;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_thermal_in_range() {
        let problem = parse("temperature 45°C is within safe range of 20°C to 80°C").unwrap();
        assert_eq!(problem.domain, "thermal");
        assert_eq!(problem.variables.len(), 3);
    }

    #[test]
    fn test_parse_thermal_missing_temp() {
        let result = parse("is within safe range of 20 to 80");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_thermal_missing_range() {
        let result = parse("temperature 45°C is fine");
        assert!(result.is_err());
    }
}

    Ok(ConstraintProblem {
        domain: "thermal".into(),
        variables: vec![
            super::Variable {
                name: "temp_c".into(),
                value: temp_c,
                desc: "temperature (°C)".into(),
            },
            super::Variable {
                name: "min_safe".into(),
                value: min_safe,
                desc: "minimum safe temp (°C)".into(),
            },
            super::Variable {
                name: "max_safe".into(),
                value: max_safe,
                desc: "maximum safe temp (°C)".into(),
            },
        ],
        constraints: vec![super::Constraint::ThermalBound {
            temp_c,
            min_safe,
            max_safe,
        }],
        assertion: super::Assertion {
            assertion_type: "in_range".into(),
            expected: 0.0,
            actual_expr: format!(
                "temperature {}°C within [{}, {}]",
                temp_c, min_safe, max_safe
            ),
        },
    })
}
