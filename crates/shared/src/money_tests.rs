use super::{CurrencyCode, Money, MoneyError};

#[test]
fn should_add_money_with_the_same_currency() {
    let usd = CurrencyCode::try_from("USD").expect("USD should be valid");
    let left = Money::new(125, usd.clone()).expect("left money should be valid");
    let right = Money::new(75, usd).expect("right money should be valid");

    let total = left
        .checked_add(right)
        .expect("same-currency addition should work");

    assert_eq!(total.minor(), 200);
    assert_eq!(total.currency().as_str(), "USD");
}

#[test]
fn should_reject_addition_for_different_currencies() {
    let usd = Money::new(
        100,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("USD money should be valid");
    let eur = Money::new(
        100,
        CurrencyCode::try_from("EUR").expect("EUR should be valid"),
    )
    .expect("EUR money should be valid");

    assert_eq!(usd.checked_add(eur), Err(MoneyError::CurrencyMismatch));
}

#[test]
fn should_reject_negative_amounts() {
    let currency = CurrencyCode::try_from("USD").expect("USD should be valid");

    assert_eq!(Money::new(-1, currency), Err(MoneyError::NegativeAmount));
}

#[test]
fn should_reject_invalid_currency_codes() {
    assert!(CurrencyCode::try_from("usd").is_err());
    assert!(CurrencyCode::try_from("US").is_err());
    assert!(CurrencyCode::try_from("US1").is_err());
}

#[test]
fn should_reject_integer_overflow() {
    let currency = CurrencyCode::try_from("USD").expect("USD should be valid");
    let left = Money::new(i64::MAX, currency.clone()).expect("left money should be valid");
    let right = Money::new(1, currency).expect("right money should be valid");

    assert_eq!(left.checked_add(right), Err(MoneyError::Overflow));
}

#[test]
fn should_multiply_money_by_a_quantity() {
    let money = Money::new(
        125,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("money should be valid");

    let total = money.checked_mul(4).expect("multiplication should work");

    assert_eq!(total.minor(), 500);
}

#[test]
fn should_reject_money_multiplication_overflow() {
    let money = Money::new(
        i64::MAX,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("money should be valid");

    assert_eq!(money.checked_mul(2), Err(MoneyError::Overflow));
}
