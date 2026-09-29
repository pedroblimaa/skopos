use super::{LoginState, LoginStep};

#[test]
fn old_generation_cannot_reset_a_new_step() {
    let mut state = LoginState::default();
    let old = state.begin(LoginStep::Qr);
    let current = state.begin(LoginStep::RequestingPhone);

    assert!(!state.reset_if(old, &LoginStep::Qr));
    assert!(state.is_step(current, &LoginStep::RequestingPhone));
}

#[test]
fn wrong_action_preserves_login_step() {
    let mut state = LoginState::default();
    let generation = state.begin(LoginStep::Phone {
        phone: "+5511999999999".into(),
        hash: "hash".into(),
    });

    assert!(state.take_password().is_err());
    assert!(state.is_step(
        generation,
        &LoginStep::Phone {
            phone: String::new(),
            hash: String::new()
        }
    ));

    assert!(state.take_phone().is_ok());
    assert!(state.is_step(generation, &LoginStep::SubmittingCode));
}

#[test]
fn sign_out_blocks_new_login() {
    let mut state = LoginState::default();
    state.begin(LoginStep::SigningOut);

    assert!(state.begin_login(LoginStep::Qr).is_err());
    assert!(matches!(state.step, LoginStep::SigningOut));
}

#[test]
fn qr_state_only_matches_current_generation() {
    let mut state = LoginState::default();
    let qr = state.begin_login(LoginStep::Qr).unwrap();

    assert!(state.is_qr(qr));
    assert!(!state.is_qr(qr.wrapping_add(1)));

    assert!(state.reset_if(qr, &LoginStep::Qr));
    assert!(!state.is_qr(qr));
    assert!(!state.reset_if(qr, &LoginStep::Qr));
}

#[test]
fn code_submission_requires_phone_step() {
    let mut state = LoginState::default();
    state.begin(LoginStep::RequestingPhone);

    assert!(state.take_phone().is_err());
    assert!(matches!(state.step, LoginStep::RequestingPhone));

    assert!(state.take_password().is_err());
    assert!(matches!(state.step, LoginStep::RequestingPhone));
}
