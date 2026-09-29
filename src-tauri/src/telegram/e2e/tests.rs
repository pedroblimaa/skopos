use super::*;

#[test]
fn qr_responses_round_trip_through_the_telegram_boundary() {
    let state = FixtureState::default();
    let request = tl::functions::auth::ExportLoginToken {
        api_id: 1,
        api_hash: "test".into(),
        except_ids: vec![],
    };
    state.0.lock().unwrap().qr_reply = Some(QrReply::Error);

    assert!(matches!(
        state.invoke(&request),
        Err(InvocationError::Dropped)
    ));

    state.0.lock().unwrap().qr_reply = Some(QrReply::PasswordRequired);

    assert!(state
        .invoke(&request)
        .unwrap_err()
        .is("SESSION_PASSWORD_NEEDED"));

    state.0.lock().unwrap().qr_reply = Some(QrReply::Authorized);

    assert!(matches!(
        state.invoke(&request).unwrap(),
        tl::enums::auth::LoginToken::Success(_)
    ));
    assert!(state.is_authorized().unwrap());
}
