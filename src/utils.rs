use rust_decimal::{prelude::*, RoundingStrategy::ToZero};

/// The number of quarks in one Kin.
const QUARKS_IN_ONE_KIN: u32 = 100_000;

/// Converts a string Kin amount to quarks.
///
/// If the provided Kin amount contains more than 5 decimal
/// places (i.e. an inexact number of quarks), additional
/// decimal places will be ignored.
///
/// For example, passing in a value of "0.000009" will result
/// in a value of 0 quarks being returned.
pub fn kin_to_quarks(amount: &str) -> u64 {
    let kin = Decimal::from_str(amount)
        .unwrap()
        .round_dp_with_strategy(5, ToZero);

    let quarks = kin * quarks_in_one_kin();

    quarks.normalize().to_u64().unwrap()
}

/// Converts an integer quark amount into a string Kin amount.
pub fn quarks_to_kin(amount: i64) -> String {
    let quarks = Decimal::new(amount, 0);

    (quarks / quarks_in_one_kin()).normalize().to_string()
}

/// Returns the number of quarks in one Kin.
fn quarks_in_one_kin() -> Decimal {
    Decimal::new(QUARKS_IN_ONE_KIN as i64, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEN_TRILLION: &str = "10000000000000";

    #[test]
    fn test_kin_to_quarks() {
        assert_eq!(kin_to_quarks("0.000009"), 0);
        assert_eq!(kin_to_quarks("0.00015"), 15);
        assert_eq!(kin_to_quarks("5"), 500_000);
        assert_eq!(kin_to_quarks("5.1"), 510_000);
        assert_eq!(kin_to_quarks("5.123459"), 512_345);
        assert_eq!(kin_to_quarks(TEN_TRILLION), 1e18 as u64);
    }

    #[test]
    fn test_quarks_to_kin() {
        assert_eq!(quarks_to_kin(15), "0.00015");
        assert_eq!(quarks_to_kin(500_000), "5");
        assert_eq!(quarks_to_kin(510_000), "5.1");
        assert_eq!(quarks_to_kin(512_345), "5.12345");
        assert_eq!(quarks_to_kin(1e18 as i64), TEN_TRILLION);
    }
}
