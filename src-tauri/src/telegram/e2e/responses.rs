use grammers_client::{sender::RpcError, tl, InvocationError};

pub(in crate::telegram) fn rpc(name: &str) -> InvocationError {
    InvocationError::Rpc(RpcError {
        code: 400,
        name: name.into(),
        value: None,
        caused_by: None,
    })
}

pub(in crate::telegram) fn authorization() -> tl::enums::auth::Authorization {
    tl::types::auth::Authorization {
        setup_password_required: false,
        otherwise_relogin_days: None,
        tmp_sessions: None,
        future_auth_token: None,
        user: tl::types::UserEmpty { id: 1 }.into(),
    }
    .into()
}

pub(in crate::telegram) fn password() -> tl::types::account::Password {
    tl::types::account::Password {
        has_recovery: false,
        has_secure_values: false,
        has_password: true,
        current_algo: Some(tl::enums::PasswordKdfAlgo::Unknown),
        srp_b: Some(vec![]),
        srp_id: Some(0),
        hint: Some("My hint".into()),
        email_unconfirmed_pattern: None,
        new_algo: tl::enums::PasswordKdfAlgo::Unknown,
        new_secure_algo: tl::enums::SecurePasswordKdfAlgo::Unknown,
        secure_random: vec![],
        pending_reset_date: None,
        login_email_pattern: None,
    }
}
