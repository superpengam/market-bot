use super::{User, UserError, UserStatus};

#[test]
fn should_register_user_with_normalized_email() {
    let user = User::register("  buyer@example.com  ".to_owned()).expect("email should be valid");

    assert_eq!(user.email(), "buyer@example.com");
    assert_eq!(user.status(), UserStatus::Active);
}

#[test]
fn should_reject_invalid_email() {
    assert_eq!(
        User::register("not-an-email".to_owned()),
        Err(UserError::InvalidEmail)
    );
}
