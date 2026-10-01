use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", content = "params", rename_all = "camelCase")]
pub enum AppMessage {
    AuthStorage,
    AuthCancelled,
    AuthNetwork,
    AuthLoginFailed,
    FloodWait,
    FloodWaitSeconds { seconds: u64 },
    InvalidPhone,
    CodeExpired,
    InvalidCode,
    IncorrectPassword,
    TelegramRejected { name: String },
    EmailSetupRequired,
    AdditionalLoginStep,
    MissingCode,
    SignUpRequired,
    PasswordVerificationFailed,
    LoginIncomplete,
    InternationalPhoneRequired,
    SignOutInProgress,
    RequestCodeFirst,
    RestartLogin,
    MissingCredentials,
    InvalidApiId,
    MissingDataCenter,
    QrMigrationFailed,
    QrLoginIncomplete,
    InvalidPhrase,
    InvalidPrice,
    ProductNotFound,
    WatchStorage,
    WatchStorageOpen,
    DeliveryApp,
    DeliverySms,
    DeliveryCall,
    DeliveryEmail { email: String },
    DeliveryFragment { url: String },
    DeliveryMissedCall { prefix: String },
    DeliveryFlashCall,
    DeliverySmsPhrase,
}

#[cfg(test)]
#[path = "app_message/tests.rs"]
mod tests;
