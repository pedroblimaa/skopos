use super::AppMessage;
use serde_json::json;

#[test]
fn serializes_static_codes_and_preserves_message_parameters() {
    let messages = [
        (AppMessage::AuthNetwork, json!({ "code": "authNetwork" })),
        (
            AppMessage::IncorrectPassword,
            json!({ "code": "incorrectPassword" }),
        ),
        (
            AppMessage::FloodWaitSeconds { seconds: 30 },
            json!({ "code": "floodWaitSeconds", "params": { "seconds": 30 } }),
        ),
        (
            AppMessage::TelegramRejected {
                name: "UNKNOWN_RPC".into(),
            },
            json!({ "code": "telegramRejected", "params": { "name": "UNKNOWN_RPC" } }),
        ),
        (
            AppMessage::DeliveryEmail {
                email: "p***@example.com".into(),
            },
            json!({ "code": "deliveryEmail", "params": { "email": "p***@example.com" } }),
        ),
        (
            AppMessage::DeliveryFragment {
                url: "https://t.me/code".into(),
            },
            json!({ "code": "deliveryFragment", "params": { "url": "https://t.me/code" } }),
        ),
        (
            AppMessage::DeliveryMissedCall {
                prefix: "+55".into(),
            },
            json!({ "code": "deliveryMissedCall", "params": { "prefix": "+55" } }),
        ),
    ];

    for (message, expected) in messages {
        assert_eq!(serde_json::to_value(message).unwrap(), expected);
    }
}
