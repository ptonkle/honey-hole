//! Decimals in the sound files, held as whole thousandths ("milli").
//!
//! `data/sounds.json` and the audio section of `data/tuning.json` are
//! written for people, so a gain reads 0.4 and a Q reads 0.7. The core does
//! no floating-point maths (BUILDER-SPEC D2), so each number is read from
//! its decimal text into thousandths (0.4 -> 400, 440 -> 440_000), and the
//! page is sent decimals written back from thousandths.

use serde_json::{Number, Value};

/// One whole unit, in thousandths.
pub const MILLI: i64 = 1000;

/// The largest magnitude a sound number may have, in whole units.
const LARGEST: i64 = 1_000_000_000;

/// A JSON number as whole thousandths: 0.25 -> 250, 440 -> 440_000.
/// `what` names the number in the refusal.
pub fn parse_milli(v: &Value, what: &str) -> Result<i64, String> {
    let Value::Number(n) = v else {
        return Err(format!("{what} is not a number."));
    };
    let text = n.to_string();
    if text.contains(['e', 'E']) {
        return Err(format!("{what} ({text}) must be written as a plain decimal."));
    }
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.as_str()),
    };
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    // serde_json writes 2.0 for a whole decimal; a trailing zero adds no place
    let frac = frac.trim_end_matches('0');
    if frac.len() > 3 {
        return Err(format!("{what} ({text}) has more than three decimal places."));
    }
    let whole: i64 = whole
        .parse()
        .ok()
        .filter(|w| *w <= LARGEST)
        .ok_or_else(|| format!("{what} ({text}) is too large."))?;
    let mut thousandths: i64 = 0;
    for (k, c) in frac.chars().enumerate() {
        let d = c
            .to_digit(10)
            .ok_or_else(|| format!("{what} ({text}) is not a decimal."))? as i64;
        thousandths += d * [100, 10, 1][k];
    }
    let v = whole * MILLI + thousandths;
    Ok(if negative { -v } else { v })
}

/// Thousandths as a JSON decimal: 250 -> 0.25.
pub fn dec(milli: i64) -> Value {
    let sign = if milli < 0 { "-" } else { "" };
    let a = milli.unsigned_abs();
    let unit = MILLI as u64;
    let text = format!("{sign}{}.{:03}", a / unit, a % unit);
    Value::Number(
        text.parse::<Number>()
            .expect("a written decimal is a JSON number"),
    )
}

/// `a * b` where both are thousandths, in thousandths.
pub fn mul(a: i64, b: i64) -> i64 {
    a * b / MILLI
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn num(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn decimals_read_as_thousandths() {
        assert_eq!(parse_milli(&json!(440), "f"), Ok(440_000));
        assert_eq!(parse_milli(&num("0.25"), "g"), Ok(250));
        assert_eq!(parse_milli(&num("-1.5"), "g"), Ok(-1500));
        assert_eq!(parse_milli(&num("2.0"), "g"), Ok(2000));
        assert_eq!(parse_milli(&num("0.007"), "g"), Ok(7));
    }

    #[test]
    fn a_number_with_too_many_places_or_no_number_is_refused_by_name() {
        let e = parse_milli(&num("0.0005"), "cue x gain").unwrap_err();
        assert!(e.starts_with("cue x gain"), "{e}");
        assert!(parse_milli(&json!("loud"), "cue x gain").is_err());
    }

    #[test]
    fn thousandths_write_back_as_decimals() {
        assert_eq!(dec(250).to_string(), "0.25");
        assert_eq!(dec(440_000).to_string(), "440.0");
        assert_eq!(dec(-1500).to_string(), "-1.5");
        assert_eq!(dec(7).to_string(), "0.007");
    }
}
