//! Fake Resend API (MAIR-522): records the e-mails Core sends instead of delivering them.
//!
//! One server per test binary on a random local port, reached through `RESEND_BASE_URL`, which
//! `resend_rs` reads when a client is built: build mailers with [`account_emails`] only.

use std::sync::{Arc, LazyLock, Mutex};

use actix_web::{web, App, HttpResponse, HttpServer};
use core_api::emails::AccountEmails;
use mairie360_api_lib::email::ResendMailer;
use serde_json::{json, Value};

/// Domain of the test mailers: the e-mails leave from `no-reply@mairie360.test`.
pub const TEST_EMAIL_DOMAIN: &str = "mairie360.test";
/// `PUBLIC_URL` of the test mailers.
pub const TEST_PUBLIC_URL: &str = "https://login.mairie360.test";

static RECEIVED: LazyLock<Arc<Mutex<Vec<Value>>>> = LazyLock::new(Arc::default);
/// Answers `500` instead of accepting the e-mails while `true`.
static FAILING: LazyLock<Arc<Mutex<bool>>> = LazyLock::new(Arc::default);

static SERVER: LazyLock<String> = LazyLock::new(|| {
    let received = Arc::clone(&RECEIVED);
    let failing = Arc::clone(&FAILING);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        actix_web::rt::System::new().block_on(async move {
            let server = HttpServer::new(move || {
                let received = Arc::clone(&received);
                let failing = Arc::clone(&failing);
                App::new().route(
                    "/emails",
                    web::post().to(move |body: web::Json<Value>| {
                        let received = Arc::clone(&received);
                        let failing = Arc::clone(&failing);
                        async move {
                            if *failing.lock().unwrap() {
                                return HttpResponse::InternalServerError().json(json!({
                                    "statusCode": 500,
                                    "name": "internal_server_error",
                                    "message": "fake Resend failure"
                                }));
                            }
                            received.lock().unwrap().push(body.into_inner());
                            HttpResponse::Ok().json(json!({ "id": "fake-resend-id" }))
                        }
                    }),
                )
            })
            .workers(1)
            .bind(("127.0.0.1", 0))
            .expect("Failed to bind the fake Resend server");
            tx.send(format!("http://127.0.0.1:{}", server.addrs()[0].port()))
                .unwrap();
            server.run().await.unwrap();
        });
    });
    let url = rx.recv().expect("the fake Resend server did not start");
    std::env::set_var("RESEND_BASE_URL", &url);
    url
});

/// The app data `forgot_password` needs, sending to the fake Resend.
#[must_use]
pub fn account_emails() -> web::Data<AccountEmails> {
    LazyLock::force(&SERVER);
    let mailer = ResendMailer::new("re_fake", TEST_EMAIL_DOMAIN, Some("Mairie 360")).unwrap();
    web::Data::new(AccountEmails::new(mailer, TEST_PUBLIC_URL, Some("Test".to_string())).unwrap())
}

/// The e-mails the fake Resend received for `to`, oldest first (request bodies of `POST /emails`).
#[must_use]
pub fn sent_to(to: &str) -> Vec<Value> {
    RECEIVED
        .lock()
        .unwrap()
        .iter()
        .filter(|body| body["to"] == json!([to]))
        .cloned()
        .collect()
}

/// Makes the fake Resend refuse every e-mail until the guard is dropped.
pub struct FailingGuard;

impl FailingGuard {
    #[must_use]
    pub fn set() -> Self {
        *FAILING.lock().unwrap() = true;
        Self
    }
}

impl Drop for FailingGuard {
    fn drop(&mut self) {
        *FAILING.lock().unwrap() = false;
    }
}

/// The reset token carried by the `RESET_URL` of a recorded password reset e-mail.
#[must_use]
pub fn reset_token(body: &Value) -> String {
    body["template"]["variables"]["RESET_URL"]
        .as_str()
        .and_then(|url| url.split_once("token="))
        .map(|(_, token)| token.to_string())
        .expect("RESET_URL without a token")
}
