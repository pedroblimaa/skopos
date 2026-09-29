use grammers_client::tl;
use serde_json::json;

use std::{collections::VecDeque, sync::Mutex};

use grammers_client::{sender::RpcError, InvocationError};

use super::{
    code_delivery,
    request::{send_code_with, CodeSender},
    CodeRequest,
};
use crate::telegram::error::{AuthError, AuthResult};

#[test]
fn formats_numeric_code_delivery_channels() {
    use tl::{enums::auth::SentCodeType, types};

    let examples = [
        (
            SentCodeType::App(types::auth::SentCodeTypeApp { length: 5 }),
            "Telegram app",
        ),
        (
            SentCodeType::Sms(types::auth::SentCodeTypeSms { length: 5 }),
            "SMS",
        ),
        (
            SentCodeType::Call(types::auth::SentCodeTypeCall { length: 5 }),
            "phone call",
        ),
        (
            SentCodeType::EmailCode(types::auth::SentCodeTypeEmailCode {
                apple_signin_allowed: false,
                google_signin_allowed: false,
                email_pattern: "p***@example.com".into(),
                length: 5,
                reset_available_period: None,
                reset_pending_date: None,
            }),
            "p***@example.com",
        ),
        (
            SentCodeType::FragmentSms(types::auth::SentCodeTypeFragmentSms {
                url: "https://t.me/code".into(),
                length: 5,
            }),
            "https://t.me/code",
        ),
        (
            SentCodeType::MissedCall(types::auth::SentCodeTypeMissedCall {
                prefix: "+55".into(),
                length: 5,
            }),
            "+55",
        ),
        (
            SentCodeType::FirebaseSms(types::auth::SentCodeTypeFirebaseSms {
                nonce: None,
                play_integrity_project_id: None,
                play_integrity_nonce: None,
                receipt: None,
                push_timeout: None,
                length: 5,
            }),
            "SMS",
        ),
    ];

    for (kind, expected_message) in examples {
        let result = code_delivery(kind);

        let value = serde_json::to_value(result).unwrap();

        assert_eq!(value["step"], "codeSent");
        assert_eq!(value["length"], 5);
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains(expected_message));
    }
}

#[test]
fn formats_text_and_external_code_delivery_channels() {
    use tl::{enums::auth::SentCodeType, types};

    let examples = [
        SentCodeType::FlashCall(types::auth::SentCodeTypeFlashCall {
            pattern: "*123".into(),
        }),
        SentCodeType::SetUpEmailRequired(types::auth::SentCodeTypeSetUpEmailRequired {
            apple_signin_allowed: false,
            google_signin_allowed: false,
        }),
        SentCodeType::SmsPhrase(types::auth::SentCodeTypeSmsPhrase { beginning: None }),
        SentCodeType::SmsWord(types::auth::SentCodeTypeSmsWord { beginning: None }),
    ];
    for kind in examples {
        let value = serde_json::to_value(code_delivery(kind)).unwrap();

        assert_eq!(value["step"], "codeSent");
        assert!(value["length"].is_null());
        assert!(!value["message"].as_str().unwrap().is_empty());
    }
}

#[test]
fn immediate_authorization_has_a_distinct_response() {
    let value = serde_json::to_value(CodeRequest::Authorized {
        status: crate::telegram::client::SessionStatus::signed_out(),
    })
    .unwrap();

    assert_eq!(
        value,
        json!({ "step": "authorized", "status": {
            "authorized": false,
            "displayName": null
        }})
    );
}

struct FakeSender {
    replies: Mutex<VecDeque<Result<tl::enums::auth::SentCode, InvocationError>>>,
    dcs: Mutex<Vec<i32>>,
    storage_error: bool,
}

impl FakeSender {
    fn new(replies: Vec<Result<tl::enums::auth::SentCode, InvocationError>>) -> Self {
        Self {
            replies: Mutex::new(replies.into()),
            dcs: Mutex::new(vec![]),
            storage_error: false,
        }
    }
}

impl CodeSender for FakeSender {
    async fn invoke(
        &self,
        request: &tl::functions::auth::SendCode,
    ) -> Result<tl::enums::auth::SentCode, InvocationError> {
        assert_eq!(request.phone_number, "+5511999999999");
        assert_eq!(request.api_id, 1);
        self.replies.lock().unwrap().pop_front().unwrap()
    }

    async fn set_home_dc(&self, dc: i32) -> AuthResult<()> {
        self.dcs.lock().unwrap().push(dc);
        if self.storage_error {
            Err(AuthError::Storage)
        } else {
            Ok(())
        }
    }
}

fn sent_code() -> tl::enums::auth::SentCode {
    tl::enums::auth::SentCode::Code(tl::types::auth::SentCode {
        r#type: tl::enums::auth::SentCodeType::App(tl::types::auth::SentCodeTypeApp { length: 5 }),
        phone_code_hash: "hash".into(),
        next_type: None,
        timeout: None,
    })
}

fn migration(value: Option<u32>) -> InvocationError {
    InvocationError::Rpc(RpcError {
        code: 303,
        name: "PHONE_MIGRATE".into(),
        value,
        caused_by: None,
    })
}

#[tokio::test]
async fn retries_phone_code_after_data_center_migration() {
    let sender = FakeSender::new(vec![Err(migration(Some(4))), Ok(sent_code())]);

    assert!(matches!(
        send_code_with(&sender, 1, "hash", "+5511999999999")
            .await
            .unwrap(),
        tl::enums::auth::SentCode::Code(_)
    ));
    assert_eq!(*sender.dcs.lock().unwrap(), [4]);
}

#[tokio::test]
async fn reports_missing_migration_and_storage_failure() {
    let sender = FakeSender::new(vec![Err(migration(None))]);

    assert!(send_code_with(&sender, 1, "hash", "+5511999999999")
        .await
        .err()
        .unwrap()
        .message()
        .contains("new data center"));

    let mut sender = FakeSender::new(vec![Err(migration(Some(4)))]);
    sender.storage_error = true;

    assert!(send_code_with(&sender, 1, "hash", "+5511999999999")
        .await
        .err()
        .unwrap()
        .message()
        .contains("local Telegram session"));
}

#[tokio::test]
async fn concrete_phone_migration_retries_and_updates_real_sqlite_session() {
    use crate::telegram::e2e::test_context;
    use grammers_session::Session;
    use tl::Serializable;

    let (context, fixture) = test_context().await;

    {
        let mut responses = fixture.0.lock().unwrap();
        responses.replies.push_back(Err(migration(Some(4))));
        responses.replies.push_back(Ok(sent_code().to_bytes()));
    }

    let response = super::send_code(&context, "+5511999999999").await.unwrap();

    assert!(
        matches!(response, tl::enums::auth::SentCode::Code(code) if code.phone_code_hash == "hash")
    );
    assert_eq!(context.session.home_dc_id().unwrap(), 4);
}
