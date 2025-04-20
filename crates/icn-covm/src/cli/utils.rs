use crate::typed::TypedValue;
use crate::typed::TypedValueError;

/// Parse a number argument with an optional default value
pub fn parse_number_arg(name: &str, raw: Option<&str>, default: f64) -> Result<TypedValue, String> {
    match raw {
        Some(s) => {
            s.parse::<f64>()
                .map(TypedValue::Number)
                .map_err(|_| format!("Invalid number for `{}`: {}", name, s))
        }
        None => Ok(TypedValue::Number(default)),
    }
}

/// Parse a boolean argument with a default value
pub fn parse_bool_arg(name: &str, raw: Option<&str>, default: bool) -> Result<TypedValue, String> {
    match raw {
        Some(s) => {
            match s.to_lowercase().as_str() {
                "true" | "yes" | "1" | "on" => Ok(TypedValue::Boolean(true)),
                "false" | "no" | "0" | "off" => Ok(TypedValue::Boolean(false)),
                _ => Err(format!("Invalid boolean for `{}`: {}", name, s)),
            }
        }
        None => Ok(TypedValue::Boolean(default)),
    }
}

/// Parse a string argument with an optional default value
pub fn parse_string_arg(name: &str, raw: Option<&str>, default: Option<&str>) -> Result<TypedValue, String> {
    match raw {
        Some(s) => Ok(TypedValue::String(s.to_string())),
        None => match default {
            Some(d) => Ok(TypedValue::String(d.to_string())),
            None => Err(format!("Missing required string parameter `{}`", name)),
        },
    }
}

/// Safely convert an f64 to a u64
pub fn safe_f64_to_u64(val: f64, operation: &str) -> Result<u64, String> {
    if val.is_nan() || val.is_infinite() || val < 0.0 || val > (u64::MAX as f64) {
        return Err(format!("Cannot convert {} to u64: value out of bounds", val));
    }
    let rounded = val.round();
    Ok(rounded as u64)
}

/// Safely convert a TypedValue to u64
pub fn safe_typed_to_u64(val: &TypedValue, operation: &str) -> Result<u64, String> {
    match val.as_number() {
        Ok(num) => safe_f64_to_u64(num, operation),
        Err(e) => Err(format!("Cannot convert to u64: {}", e)),
    }
}

/// Convert f64 to TypedValue::Number 
pub fn f64_to_typed(val: f64) -> TypedValue {
    TypedValue::Number(val)
}

/// Calculate a percentage safely from a TypedValue
pub fn safe_percentage(numerator: &TypedValue, denominator: &TypedValue) -> Result<f64, String> {
    let num = numerator.as_number().map_err(|e| format!("Invalid numerator: {}", e))?;
    let denom = denominator.as_number().map_err(|e| format!("Invalid denominator: {}", e))?;
    
    if denom == 0.0 {
        return Err("Cannot calculate percentage with zero denominator".to_string());
    }
    
    Ok((num / denom) * 100.0)
} 