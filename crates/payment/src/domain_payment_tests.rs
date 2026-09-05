use market_bot_shared::{CurrencyCode, Money, OrderId};

use super::{Payment, PaymentStatus};

fn amount(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

#[test]
fn should_transition_payment_from_created_to_succeeded() {
    let mut payment = Payment::new(OrderId::new(), amount(1_500));

    payment
        .transition_to(PaymentStatus::Processing)
        .expect("payment should start processing");
    payment
        .transition_to(PaymentStatus::Succeeded)
        .expect("payment should succeed");

    assert_eq!(payment.status(), PaymentStatus::Succeeded);
}

#[test]
fn should_reject_payment_transition_back_to_created() {
    let mut payment = Payment::new(OrderId::new(), amount(1_500));

    payment
        .transition_to(PaymentStatus::Processing)
        .expect("payment should start processing");

    assert!(payment.transition_to(PaymentStatus::Created).is_err());
}

#[test]
fn should_allow_a_succeeded_payment_to_move_to_refunded() {
    let mut payment = Payment::new(OrderId::new(), amount(1_500));
    payment
        .transition_to(PaymentStatus::Processing)
        .expect("payment should start processing");
    payment
        .transition_to(PaymentStatus::Succeeded)
        .expect("payment should succeed");

    payment
        .transition_to(PaymentStatus::Refunded)
        .expect("provider-confirmed refund can complete from succeeded");

    assert_eq!(payment.status(), PaymentStatus::Refunded);
}
