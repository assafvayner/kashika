/// `v` rounded to 2 decimals, never `-0`.
pub fn round2(v: f64) -> f64 {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 { 0.0 } else { r }
}

/// `v` as a person would write it: at most 2 decimals, no trailing `.0` and never `-0`.
/// Values too large to round are written as they are.
pub fn fmt_num(v: f64) -> String {
    let r = round2(v);
    format!("{}", if r.is_finite() { r } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_two_decimals_without_negative_zero() {
        assert_eq!(fmt_num(1.005_1), "1.01");
        assert_eq!(fmt_num(12.0), "12");
        assert_eq!(fmt_num(-0.001), "0");
        assert_eq!(fmt_num(-2.5), "-2.5");
        assert_eq!(round2(-0.004), 0.0);
        assert!(round2(-0.004).is_sign_positive());
    }

    #[test]
    fn writes_non_finite_and_huge_values_as_they_are() {
        assert_eq!(fmt_num(f64::INFINITY), "inf");
        assert_eq!(fmt_num(f64::NAN), "NaN");
        assert_eq!(fmt_num(f64::MAX), format!("{}", f64::MAX));
    }
}
