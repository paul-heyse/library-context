//! Value presentation never evaluates source, defaults or factories.
use super::Literal;
use crate::domain::{
    ModelError,
    resources::{Reservation, ResourceBudget},
};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Human,
    CapturedSource,
    PythonExpression,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    MissingSource,
    NonFiniteExpression,
}
/// The rendering allocation remains admitted while its consumer copies the presentation.
pub struct Rendered {
    pub text: String,
    _reservation: Box<dyn Reservation>,
}
pub fn render(
    literal: &Literal,
    mode: Mode,
    source: Option<&str>,
    budget: &ResourceBudget,
) -> Result<Result<Rendered, Unavailable>, ModelError> {
    let input_bytes = if mode == Mode::CapturedSource {
        let Some(source) = source else {
            return Ok(Err(Unavailable::MissingSource));
        };
        source.len()
    } else {
        match literal {
            Literal::Integer { decimal } => decimal.len(),
            Literal::String { value } => value.len(),
            Literal::Bytes { value } => value.0.len(),
            _ => 32,
        }
    };
    let allowance = input_bytes
        .checked_mul(6)
        .and_then(|n| n.checked_add(128))
        .ok_or_else(|| ModelError::Invalid("value presentation size overflow".into()))?;
    let reservation = budget.reserve("literal_presentation", allowance)?;
    let text = if mode == Mode::CapturedSource {
        source.expect("source checked above").to_owned()
    } else {
        match literal {
            Literal::None => "None".into(),
            Literal::Bool { value } => if *value { "True" } else { "False" }.into(),
            Literal::Integer { decimal } => {
                // CPython permits a decimal digit limit as low as 640. Hexadecimal integer
                // literals are exempt, so large expressions remain executable under that policy.
                if mode == Mode::PythonExpression && decimal.len() > 640 {
                    let integer = num_bigint::BigInt::parse_bytes(decimal.as_bytes(), 10)
                        .ok_or_else(|| {
                            ModelError::Invalid("invalid integer presentation".into())
                        })?;
                    let digits = integer.to_str_radix(16);
                    if let Some(digits) = digits.strip_prefix('-') {
                        format!("-0x{digits}")
                    } else {
                        format!("0x{digits}")
                    }
                } else {
                    decimal.clone()
                }
            }
            Literal::String { value } => serde_json::to_string(value.as_str())
                .map_err(|e| ModelError::Codec(e.to_string()))?,
            Literal::Bytes { value } => {
                let mut text = String::with_capacity(value.0.len() * 4 + 3);
                text.push_str("b\"");
                for byte in &value.0 {
                    write!(text, "\\x{byte:02x}").expect("string write");
                }
                text.push('"');
                text
            }
            Literal::Float { bits } => {
                let value = f64::from_bits(*bits as u64);
                if !value.is_finite() {
                    if mode == Mode::PythonExpression {
                        return Ok(Err(Unavailable::NonFiniteExpression));
                    }
                    let name = if value.is_nan() {
                        "NaN"
                    } else if value.is_sign_negative() {
                        "-Infinity"
                    } else {
                        "+Infinity"
                    };
                    format!("{name} (IEEE-754 bits 0x{:016x})", *bits as u64)
                } else {
                    let mut text = value.to_string();
                    if !text.contains(['.', 'e', 'E']) {
                        text.push_str(".0");
                    }
                    text
                }
            }
        }
    };
    Ok(Ok(Rendered {
        text,
        _reservation: reservation,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EvidenceBytes, Utf8Text};
    #[test]
    fn exact_values_modes_and_unavailable_expressions_are_distinct() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cases = [
            (
                Literal::Integer {
                    decimal: "1234567890123456789012345678901234567890".into(),
                },
                "1234567890123456789012345678901234567890",
            ),
            (
                Literal::String {
                    value: Utf8Text::from("quote\"\n\0 café 😀"),
                },
                "\"quote\\\"\\n\\u0000 café 😀\"",
            ),
            (
                Literal::Bytes {
                    value: EvidenceBytes(vec![0, 34, 92, 128, 255]),
                },
                "b\"\\x00\\x22\\x5c\\x80\\xff\"",
            ),
            (
                Literal::Float {
                    bits: (-0.0f64).to_bits() as i64,
                },
                "-0.0",
            ),
            (
                Literal::Float {
                    bits: 1.5f64.to_bits() as i64,
                },
                "1.5",
            ),
        ];
        for (literal, expected) in cases {
            let expression = render(&literal, Mode::PythonExpression, None, &budget)
                .unwrap()
                .unwrap();
            assert_eq!(expression.text, expected);
            let original = render(
                &literal,
                Mode::CapturedSource,
                Some("0xFF # original"),
                &budget,
            )
            .unwrap()
            .unwrap();
            assert_eq!(original.text, "0xFF # original");
            assert!(matches!(
                render(&literal, Mode::CapturedSource, None, &budget).unwrap(),
                Err(Unavailable::MissingSource)
            ));
        }
        let bits = [
            0x7ff0000000000000u64,
            0xfff0000000000000,
            0x7ff8000000000001,
            0x7ff8000000000002,
        ];
        let mut displays = std::collections::BTreeSet::new();
        for bits in bits {
            let literal = Literal::Float { bits: bits as i64 };
            assert!(matches!(
                render(&literal, Mode::PythonExpression, None, &budget).unwrap(),
                Err(Unavailable::NonFiniteExpression)
            ));
            displays.insert(
                render(&literal, Mode::Human, None, &budget)
                    .unwrap()
                    .unwrap()
                    .text,
            );
        }
        assert_eq!(displays.len(), 4);
        let refused = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            render(&Literal::None, Mode::Human, None, &refused),
            Err(ModelError::Resource { .. })
        ));
    }
    #[test]
    fn large_signed_integer_expressions_execute_under_python_decimal_limits() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let decimal = format!("1{}", "0".repeat(5000));
        let mut expressions = Vec::new();
        for decimal in [decimal.clone(), format!("-{decimal}")] {
            let literal = Literal::Integer {
                decimal: decimal.clone(),
            };
            assert_eq!(
                render(&literal, Mode::Human, None, &budget)
                    .unwrap()
                    .unwrap()
                    .text,
                decimal
            );
            let expression = render(&literal, Mode::PythonExpression, None, &budget)
                .unwrap()
                .unwrap();
            assert!(expression.text.starts_with("0x") || expression.text.starts_with("-0x"));
            expressions.push(expression.text);
        }
        let encoded = serde_json::to_string(&expressions).unwrap();
        let output = std::process::Command::new("uv").args(["run", "--no-sync", "python", "-I", "-c",
            "import json,sys; sys.set_int_max_str_digits(640); values=json.loads(sys.argv[1]); assert eval(values[0], {'__builtins__': {}})==10**5000; assert eval(values[1], {'__builtins__': {}})==-(10**5000)\ntry:\n compile('1'+'0'*5000, '<control>', 'eval')\nexcept SyntaxError:\n pass\nelse:\n raise AssertionError('decimal rejection control did not reject')", &encoded]).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
