//! Monitoring notifications and active alerts.

mod common;

use std::sync::Arc;

use ntex::http::Method;
use ntex::service::map_state_factory;
use ntex::web::test::{self as web_test, TestServer};
use ntex::web::App;
use server_box_monitor::api::auth::generate_token;
use server_box_monitor::api::notifications::{self, Level, Source};
use server_box_monitor::api::server::{Agent, AppState};
use server_box_monitor::core::config::Config;
use server_box_monitor::monitoring::rules::Firing;

const SECRET: &str = "test-secret-that-is-long-enough-32ch";

async fn state() -> Arc<AppState> {
    state_over(common::database().await).await
}

async fn state_over(db: sqlx::SqlitePool) -> Arc<AppState> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = Config {
        jwt_secret: Some(SECRET.to_string()),
        ..Default::default()
    };
    common::add_owner(&db).await;
    AppState::new(Arc::new(config), db)
}

async fn server(state: Arc<AppState>) -> TestServer {
    web_test::server(async move |_| {
        let state = state.clone();
        map_state_factory(
            Agent(state),
            App::<Agent>::new().configure(server_box_monitor::api::server::configure_api),
        )
    })
}

/// One request as [user], and its status and body (JSON, or the raw bytes as
/// a JSON string of their length when they are not JSON).
async fn call(
    srv: &TestServer,
    signed: bool,
    method: Method,
    path: &str,
    body: Option<Vec<u8>>,
) -> (u16, serde_json::Value, Vec<u8>) {
    let mut req = srv
        .request(method, srv.url(path))
        .timeout(std::time::Duration::from_secs(30));
    if signed {
        req = req.header(
            "Authorization",
            format!("Bearer {}", generate_token(SECRET).unwrap()),
        );
    }
    let resp = match body {
        Some(body) => req
            .header("content-type", "application/json")
            .send_body(body)
            .await
            .unwrap(),
        None => req.send().await.unwrap(),
    };
    let status = resp.status().as_u16();
    let bytes = resp
        .body()
        .limit(16 << 20)
        .await
        .unwrap_or_default()
        .to_vec();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json, bytes)
}

fn json(value: serde_json::Value) -> Option<Vec<u8>> {
    Some(value.to_string().into_bytes())
}

#[ntex::test]
async fn notifications_are_marked_read() {
    let state = state().await;
    let srv = server(state.clone()).await;
    for i in 0..3 {
        state
            .notifications
            .notify(
                &state.db,
                Level::Warning,
                Source::Alert,
                "High CPU",
                &format!("cpu {i}"),
                None,
            )
            .await
            .unwrap();
    }
    let (_, body, _) = call(&srv, true, Method::GET, "/api/v1/notifications", None).await;
    let (_, legacy, _) = call(&srv, true, Method::GET, "/api/v1/desk/notifications", None).await;
    assert_eq!(legacy, body);
    assert_eq!(body["unread"], 3);
    let list = body["notifications"].as_array().unwrap();
    assert_eq!(list[0]["body"], "cpu 2", "newest first");
    let newest = list[0]["id"].as_i64().unwrap();

    let (status, _, _) = call(
        &srv,
        true,
        Method::POST,
        "/api/v1/notifications/read",
        json(serde_json::json!({ "ids": [newest, 999_999] })),
    )
    .await;
    assert_eq!(status, 204);
    let (_, body, _) = call(&srv, true, Method::GET, "/api/v1/notifications", None).await;
    assert_eq!(body["unread"], 2);
    assert_eq!(body["notifications"][0]["read"], true);

    call(
        &srv,
        true,
        Method::POST,
        "/api/v1/notifications/read",
        json(serde_json::json!({ "all": true })),
    )
    .await;
    let (_, body, _) = call(&srv, true, Method::GET, "/api/v1/notifications", None).await;
    assert_eq!(body["unread"], 0);
}

fn cpu_over(value: &str) -> Firing {
    Firing {
        rule: "High CPU".into(),
        monitor_type: "cpu".into(),
        matcher: ">".into(),
        threshold: "90%".into(),
        value: value.into(),
        message: format!("Alert: High CPU - > {value} (threshold: 90%)"),
    }
}

#[ntex::test]
async fn a_rule_notifies_when_it_starts_firing_not_every_cycle() {
    let state = state().await;
    state
        .notifications
        .rules_checked(&state.db, vec![cpu_over("95.00%")])
        .await;
    state
        .notifications
        .rules_checked(&state.db, vec![cpu_over("96.00%")])
        .await;
    state.notifications.rules_checked(&state.db, vec![]).await;
    state
        .notifications
        .rules_checked(&state.db, vec![cpu_over("97.00%")])
        .await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM desk_notification")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

/// `/alerts`: what fires now, with the last reading and since when it has
/// been firing without a break.
#[ntex::test]
async fn alerts_say_what_fires_now_and_since_when() {
    let state = state().await;
    let srv = server(state.clone()).await;

    let (status, body, _) = call(&srv, true, Method::GET, "/api/v1/alerts", None).await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        serde_json::json!({ "checked_at": null, "alerts": [] })
    );
    let (status, _, _) = call(&srv, false, Method::GET, "/api/v1/alerts", None).await;
    assert_eq!(status, 401);

    state
        .notifications
        .rules_checked(&state.db, vec![cpu_over("95.00%")])
        .await;
    let (_, first, _) = call(&srv, true, Method::GET, "/api/v1/alerts", None).await;
    let since = first["alerts"][0]["since"].clone();
    assert_eq!(first["checked_at"], since);

    state
        .notifications
        .rules_checked(&state.db, vec![cpu_over("96.00%")])
        .await;
    let (_, body, _) = call(&srv, true, Method::GET, "/api/v1/alerts", None).await;
    assert_eq!(
        body["alerts"],
        serde_json::json!([{
            "rule": "High CPU",
            "monitor_type": "cpu",
            "matcher": ">",
            "threshold": "90%",
            "value": "96.00%",
            "since": since,
        }]),
    );

    state.notifications.rules_checked(&state.db, vec![]).await;
    let (_, body, _) = call(&srv, true, Method::GET, "/api/v1/alerts", None).await;
    assert_eq!(body["alerts"], serde_json::json!([]));
    assert!(body["checked_at"].is_string());
}

#[ntex::test]
async fn old_notifications_make_room() {
    let state = state().await;
    for i in 0..(notifications::KEEP_NOTIFICATIONS + 5) {
        state
            .notifications
            .notify(
                &state.db,
                Level::Info,
                Source::Alert,
                "r",
                &i.to_string(),
                None,
            )
            .await
            .unwrap();
    }
    let (count, oldest): (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), (SELECT body FROM desk_notification ORDER BY id LIMIT 1) FROM desk_notification",
    )
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(count, notifications::KEEP_NOTIFICATIONS);
    assert_eq!(oldest, "5");
}
