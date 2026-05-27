//! Shared parsing utilities for extracting numbers and patterns from natural language.

/// Extract a number that appears immediately before the given keyword.
pub fn extract_number_before(text: &str, keyword: &str) -> Option<f64> {
    let idx = text.find(keyword)?;
    let prefix = &text[..idx];
    let num_str = prefix
        .rsplit(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .next()?;
    num_str.parse().ok()
}

/// Extract a number near a keyword (within 30 chars before it).
pub fn extract_number_near(text: &str, keyword: &str) -> Option<f64> {
    let idx = text.find(keyword)?;
    let start = idx.saturating_sub(30);
    let window = &text[start..idx];
    // Find the last number in the window
    let parts: Vec<&str> = window
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter(|s| !s.is_empty())
        .collect();
    parts.last().and_then(|s| s.parse().ok())
}

/// Extract a number with a unit suffix (e.g., "50khz", "200hz").
pub fn extract_number_with_unit(text: &str, units: &[&str]) -> Option<f64> {
    for unit in units {
        for part in text.split_whitespace() {
            if let Some(num_str) = part.strip_suffix(unit) {
                if let Ok(v) = num_str.parse() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Extract a range from patterns like "X to Y", "between X and Y".
pub fn extract_range(text: &str) -> Option<(f64, f64)> {
    // Pattern: "between X and Y" or "range of X to Y" or "X to Y"
    if let Some(rest) = text.strip_prefix("between ") {
        let parts: Vec<&str> = rest.split(" and ").collect();
        if parts.len() == 2 {
            let a = parts[0].trim().parse::<f64>().ok()?;
            let b = parts[1].split_whitespace().next()?.parse::<f64>().ok()?;
            return Some((a.min(b), a.max(b)));
        }
    }

    // Look for "safe range" or "range of X to Y" or "from X to Y"
    // First try: extract numbers around "to" after "range" or "safe range"
    if let Some(idx) = text.find("safe range") {
        let rest = &text[idx + 10..];
        // Remove non-numeric noise like degree symbols
        let clean = rest
            .replace("°c", " ")
            .replace("°", " ")
            .replace("celsius", " ");
        if let Some(range) = extract_range_from_text(clean.trim()) {
            return Some(range);
        }
    }
    if let Some(idx) = text.find("range of ") {
        let rest = &text[idx + 9..];
        let clean = rest
            .replace("°c", " ")
            .replace("°", " ")
            .replace("celsius", " ");
        if let Some(range) = extract_range_from_text(clean.trim()) {
            return Some(range);
        }
    }

    for pattern in &["from "] {
        if let Some(idx) = text.find(pattern) {
            let rest = &text[idx + pattern.len()..];
            let parts: Vec<&str> = rest.split(" to ").collect();
            if parts.len() >= 2 {
                let a = parts[0].trim().parse::<f64>().ok()?;
                let b = parts[1].split_whitespace().next()?.parse::<f64>().ok()?;
                return Some((a.min(b), a.max(b)));
            }
        }
    }

    None
}

fn extract_range_from_text(text: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = text.split(" to ").collect();
    if parts.len() >= 2 {
        let a = parts[0].trim().parse::<f64>().ok()?;
        let b = parts[1]
            .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
            .next()?
            .parse::<f64>()
            .ok()?;
        return Some((a.min(b), a.max(b)));
    }
    // Try "X - Y" with spaces around dash
    let parts: Vec<&str> = text.split(" - ").collect();
    if parts.len() >= 2 {
        let a = parts[0].trim().parse::<f64>().ok()?;
        let b = parts[1]
            .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
            .next()?
            .parse::<f64>()
            .ok()?;
        return Some((a.min(b), a.max(b)));
    }
    None
}

/// Extract a comparison: "X is greater than Y", "X > Y", "X is at least Y"
pub fn extract_comparison(text: &str) -> Option<(f64, String, f64, String)> {
    // Direct operator: "X > Y", "X >= Y", "X < Y", "X <= Y", "X == Y"
    for op in &[">=", "<=", ">", "<", "==", "="] {
        if let Some(idx) = text.find(op) {
            let left_str = text[..idx].trim();
            let right_str = text[idx + op.len()..].trim();
            let left = left_str.split_whitespace().last()?.parse::<f64>().ok()?;
            let right = right_str.split_whitespace().next()?.parse::<f64>().ok()?;
            let op_name = match *op {
                ">=" => "gte",
                "<=" => "lte",
                ">" => "gt",
                "<" => "lt",
                "==" | "=" => "eq",
                _ => "gt",
            };
            return Some((left, op_name.to_string(), right, text.to_string()));
        }
    }

    // Natural language: "X is greater than Y", "X is at least Y", etc.
    let patterns = [
        ("greater than or equal to", "gte"),
        ("less than or equal to", "lte"),
        ("at least", "gte"),
        ("at most", "lte"),
        ("greater than", "gt"),
        ("less than", "lt"),
        ("equal to", "eq"),
        ("equals", "eq"),
        ("is above", "gt"),
        ("is below", "lt"),
    ];

    for (phrase, op) in &patterns {
        if let Some(idx) = text.find(phrase) {
            let left_str = text[..idx].trim();
            let right_str = text[idx + phrase.len()..].trim();
            // Try to extract numbers
            let left = extract_trailing_number(left_str)?;
            let right = extract_leading_number(right_str)?;
            return Some((left, op.to_string(), right, text.to_string()));
        }
    }

    None
}

fn extract_trailing_number(text: &str) -> Option<f64> {
    let parts: Vec<&str> = text
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter(|s| !s.is_empty())
        .collect();
    parts.last().and_then(|s| s.parse().ok())
}

fn extract_leading_number(text: &str) -> Option<f64> {
    let parts: Vec<&str> = text
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter(|s| !s.is_empty())
        .collect();
    parts.first().and_then(|s| s.parse().ok())
}

/// Extract a range check: "X is between Y and Z"
pub fn extract_range_check(text: &str) -> Option<(f64, f64, f64, String)> {
    if let Some(idx) = text.find("between ") {
        let rest = &text[idx + 8..];
        let parts: Vec<&str> = rest.split(" and ").collect();
        if parts.len() >= 2 {
            let min = parts[0].trim().parse::<f64>().ok()?;
            let right = parts[1].split_whitespace().collect::<Vec<_>>();
            let max = right.first()?.parse::<f64>().ok()?;
            // Find the value being checked — look before "between"
            let prefix = &text[..idx];
            let value = extract_trailing_number(prefix)?;
            return Some((value, min, max, text.to_string()));
        }
    }
    // "X is within [Y, Z]" or "X is in [Y, Z]"
    for phrase in &["within ", "in "] {
        if let Some(idx) = text.find(phrase) {
            let rest = &text[idx + phrase.len()..];
            let clean = rest.trim_start_matches(['[', '(']);
            let parts: Vec<&str> = clean.split([',', ']']).collect();
            if parts.len() >= 2 {
                let min = parts[0].trim().parse::<f64>().ok()?;
                let max = parts[1].trim().parse::<f64>().ok()?;
                let prefix = &text[..idx];
                let value = extract_trailing_number(prefix)?;
                return Some((value, min, max, text.to_string()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_number_before_found() {
        assert_eq!(extract_number_before("the depth is 200m depth", "m depth"), Some(200.0));
    }

    #[test]
    fn test_extract_number_before_keyword_absent() {
        assert_eq!(extract_number_before("no keyword here", "depth"), None);
    }

    #[test]
    fn test_extract_number_near_found() {
        assert_eq!(extract_number_near("temperature is 15.5 degrees", "degrees"), Some(15.5));
    }

    #[test]
    fn test_extract_number_with_unit_khz() {
        assert_eq!(extract_number_with_unit("a 50khz signal", &["khz"]), Some(50.0));
    }

    #[test]
    fn test_extract_number_with_unit_hz() {
        assert_eq!(extract_number_with_unit("1000hz tone", &["hz"]), Some(1000.0));
    }

    #[test]
    fn test_extract_number_with_unit_not_found() {
        assert_eq!(extract_number_with_unit("no units here", &["khz"]), None);
    }

    #[test]
    fn test_extract_range_between_and() {
        let (min, max) = extract_range("between 20 and 80").unwrap();
        assert_eq!(min, 20.0);
        assert_eq!(max, 80.0);
    }

    #[test]
    fn test_extract_range_safe_range() {
        let (min, max) = extract_range("safe range of 10 to 90").unwrap();
        assert_eq!(min, 10.0);
        assert_eq!(max, 90.0);
    }

    #[test]
    fn test_extract_range_from_to() {
        let (min, max) = extract_range("from 0 to 100").unwrap();
        assert_eq!(min, 0.0);
        assert_eq!(max, 100.0);
    }

    #[test]
    fn test_extract_range_not_found() {
        assert_eq!(extract_range("no range here"), None);
    }

    #[test]
    fn test_extract_comparison_operator_gt() {
        let (left, op, right, _) = extract_comparison("10 > 5").unwrap();
        assert_eq!(left, 10.0);
        assert_eq!(op, "gt");
        assert_eq!(right, 5.0);
    }

    #[test]
    fn test_extract_comparison_gte_symbol() {
        let (_, op, _, _) = extract_comparison("10 >= 5").unwrap();
        assert_eq!(op, "gte");
    }

    #[test]
    fn test_extract_comparison_lte_symbol() {
        let (_, op, _, _) = extract_comparison("3 <= 10").unwrap();
        assert_eq!(op, "lte");
    }

    #[test]
    fn test_extract_comparison_natural_greater() {
        let (left, op, right, _) = extract_comparison("10 is greater than 5").unwrap();
        assert_eq!(left, 10.0);
        assert_eq!(op, "gt");
        assert_eq!(right, 5.0);
    }

    #[test]
    fn test_extract_comparison_natural_at_least() {
        let (_, op, _, _) = extract_comparison("10 is at least 5").unwrap();
        assert_eq!(op, "gte");
    }

    #[test]
    fn test_extract_comparison_natural_at_most() {
        let (_, op, _, _) = extract_comparison("5 is at most 10").unwrap();
        assert_eq!(op, "lte");
    }

    #[test]
    fn test_extract_comparison_natural_equal() {
        let (_, op, _, _) = extract_comparison("5 is equal to 5").unwrap();
        assert_eq!(op, "eq");
    }

    #[test]
    fn test_extract_comparison_not_found() {
        assert_eq!(extract_comparison("hello world"), None);
    }

    #[test]
    fn test_extract_range_check_between() {
        let (value, min, max, _) = extract_range_check("50 is between 20 and 80").unwrap();
        assert_eq!(value, 50.0);
        assert_eq!(min, 20.0);
        assert_eq!(max, 80.0);
    }

    #[test]
    fn test_extract_range_check_within_brackets() {
        let (value, min, max, _) = extract_range_check("50 is within [20, 80]").unwrap();
        assert_eq!(value, 50.0);
        assert_eq!(min, 20.0);
        assert_eq!(max, 80.0);
    }

    #[test]
    fn test_extract_range_check_not_found() {
        assert_eq!(extract_range_check("nothing here"), None);
    }

    #[test]
    fn test_extract_bound_within_of() {
        let (value, min, max, _) = extract_bound("52 is within 3 of 50").unwrap();
        assert_eq!(value, 52.0);
        assert_eq!(min, 47.0);
        assert_eq!(max, 53.0);
    }

    #[test]
    fn test_extract_bound_not_found() {
        assert_eq!(extract_bound("no bound here"), None);
    }
}

/// Extract a bound: "X is within Y of Z" → (X, Z-Y, Z+Y)
pub fn extract_bound(text: &str) -> Option<(f64, f64, f64, String)> {
    if let Some(idx) = text.find("within ") {
        let rest = &text[idx + 7..];
        let parts: Vec<&str> = rest.split(" of ").collect();
        if parts.len() >= 2 {
            let tolerance = parts[0].trim().parse::<f64>().ok()?;
            let center = extract_leading_number(parts[1])?;
            let prefix = &text[..idx];
            let value = extract_trailing_number(prefix)?;
            return Some((
                value,
                center - tolerance,
                center + tolerance,
                text.to_string(),
            ));
        }
    }
    None
}
