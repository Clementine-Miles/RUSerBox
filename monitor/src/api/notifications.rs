//! Current monitoring alerts and notification history for authenticated clients.

use crate::api::WebError;
use crate::utils::lock::LockExt;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ntex::web::{self, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{authz, server::AppState};
use crate::monitoring::rules::Firing;

pub const KEEP_NOTIFICATIONS: i64 = 500;

#[derive(Default)]
pub struct NotificationHub {
    /// The monitoring rules firing at the last check: a notification is for a
    /// rule that starts, not for every cycle it stays over.
    firing: Mutex<FiringNow>,
}

/// What `/alerts` answers. In memory: an agent that restarts knows again at
/// its first check, and a rule firing across the restart starts again.
#[derive(Debug, Clone, Default, Serialize)]
pub struct FiringNow {
    /// When the rules were last checked; absent before the first check.
    pub checked_at: Option<String>,
    /// In the configured rules' order.
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    #[serde(flatten)]
    pub firing: Firing,
    /// The first check it was found firing at, without a check in between
    /// that found it not.
    pub since: String,
}

impl NotificationHub {
    /// The rules firing now: a notification for each that was not firing at
    /// the last check, and what `/alerts` answers until the next.
    pub async fn rules_checked(&self, db: &SqlitePool, firing: Vec<Firing>) {
        let now = chrono::Utc::now().to_rfc3339();
        let started: Vec<Firing> = {
            let mut state = self.firing.locked();
            let mut was: HashMap<String, String> = state
                .alerts
                .drain(..)
                .map(|alert| (alert.firing.rule, alert.since))
                .collect();
            let mut started = Vec::new();
            for fired in firing {
                let since = match was.remove(&fired.rule) {
                    Some(since) => since,
                    None => {
                        started.push(fired.clone());
                        now.clone()
                    }
                };
                state.alerts.push(Alert {
                    firing: fired,
                    since,
                });
            }
            state.checked_at = Some(now);
            started
        };
        for fired in started {
            if let Err(e) = self
                .notify(
                    db,
                    Level::Warning,
                    Source::Alert,
                    &fired.rule,
                    &fired.message,
                    None,
                )
                .await
            {
                tracing::warn!("notification for rule '{}' not stored: {e}", fired.rule);
            }
        }
    }

    /// The rules firing at the last check.
    pub fn firing(&self) -> FiringNow {
        self.firing.locked().clone()
    }

    pub async fn notify(
        &self,
        db: &SqlitePool,
        level: Level,
        source: Source,
        subject: &str,
        body: &str,
        kind: Option<(&str, &serde_json::Value)>,
    ) -> Result<(), sqlx::Error> {
        let created_at = chrono::Utc::now().to_rfc3339();
        let (kind, params) = kind
            .map(|(kind, params)| (Some(kind.to_string()), Some(params)))
            .unwrap_or((None, None));
        let id = sqlx::query(
            "INSERT INTO desk_notification (created_at, level, source, subject, body, kind, params) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&created_at)
        .bind(level.as_str())
        .bind(source.as_str())
        .bind(subject)
        .bind(body)
        .bind(&kind)
        .bind(params.map(serde_json::Value::to_string))
        .execute(db)
        .await?
        .last_insert_rowid();
        sqlx::query("DELETE FROM desk_notification WHERE id <= ?")
            .bind(id - KEEP_NOTIFICATIONS)
            .execute(db)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Critical,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Alert,
    /// A disk, array, pool or maintenance task (`storage_watch`):
    /// `storage:<what>`.
    Storage(String),
    /// The agent itself: a newer release (`api::update`).
    Agent,
    /// Backups of the monitor's settings (`monitor_backup`).
    Backup,
}

impl Source {
    fn as_str(&self) -> String {
        match self {
            Source::Alert => "alert".into(),
            Source::Storage(what) => format!("storage:{what}"),
            Source::Agent => "agent".into(),
            Source::Backup => "backup".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Notification {
    pub id: i64,
    pub created_at: String,
    pub level: String,
    pub source: String,
    pub subject: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    pub read: bool,
}

#[derive(Deserialize)]
pub struct NotificationQuery {
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    #[serde(default)]
    ids: Vec<i64>,
    #[serde(default)]
    all: bool,
}

/// `GET /alerts`: the rules firing at the last check, see [`FiringNow`].
pub async fn alerts(req: HttpRequest, state: Arc<AppState>) -> Result<HttpResponse, WebError> {
    require!(authz::jwt_caller(&req, &state).await);
    Ok(HttpResponse::Ok().json(&state.notifications.firing()))
}

pub async fn notifications(
    req: HttpRequest,
    query: web::types::Query<NotificationQuery>,
    state: Arc<AppState>,
) -> Result<HttpResponse, WebError> {
    let user = require!(authz::jwt_caller(&req, &state).await).id;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let rows = sqlx::query_as::<_, (i64, String, String, String, String, String, Option<String>, Option<String>, bool)>(
        "SELECT n.id, n.created_at, n.level, n.source, n.subject, n.body, n.kind, n.params, r.user_id IS NOT NULL \
         FROM desk_notification n \
         LEFT JOIN desk_notification_read r ON r.notification_id = n.id AND r.user_id = ? \
         ORDER BY n.id DESC LIMIT ?",
    )
    .bind(user)
    .bind(limit)
    .fetch_all(&state.db)
    .await;
    let unread = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM desk_notification n WHERE NOT EXISTS \
         (SELECT 1 FROM desk_notification_read r WHERE r.notification_id = n.id AND r.user_id = ?)",
    )
    .bind(user)
    .fetch_one(&state.db)
    .await;
    match (rows, unread) {
        (Ok(rows), Ok(unread)) => {
            let notifications: Vec<Notification> = rows
                .into_iter()
                .map(
                    |(id, created_at, level, source, subject, body, kind, params, read)| {
                        Notification {
                            id,
                            created_at,
                            level,
                            source,
                            subject,
                            body,
                            kind,
                            params: params.and_then(|params| serde_json::from_str(&params).ok()),
                            read,
                        }
                    },
                )
                .collect();
            Ok(HttpResponse::Ok().json(&serde_json::json!({
                "notifications": notifications,
                "unread": unread,
            })))
        }
        (Err(e), _) | (_, Err(e)) => Ok(super::respond::internal(&e)),
    }
}

pub async fn mark_read(
    req: HttpRequest,
    body: web::types::Json<ReadRequest>,
    state: Arc<AppState>,
) -> Result<HttpResponse, WebError> {
    let user = require!(authz::jwt_caller(&req, &state).await).id;
    let body = body.into_inner();
    if body.ids.len() > 200 {
        return Ok(HttpResponse::BadRequest().json(&serde_json::json!({ "error": "tooMany" })));
    }
    let done = if body.all {
        sqlx::query(
            "INSERT OR IGNORE INTO desk_notification_read (user_id, notification_id) \
             SELECT ?, id FROM desk_notification",
        )
        .bind(user)
        .execute(&state.db)
        .await
        .map(|_| ())
    } else {
        mark_ids(&state.db, user, &body.ids).await
    };
    match done {
        Ok(()) => Ok(HttpResponse::NoContent().build()),
        Err(e) => Ok(super::respond::internal(&e)),
    }
}

async fn mark_ids(db: &SqlitePool, user: i64, ids: &[i64]) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for id in ids {
        // An id that is gone (or never was) marks nothing.
        sqlx::query(
            "INSERT OR IGNORE INTO desk_notification_read (user_id, notification_id) \
             SELECT ?, id FROM desk_notification WHERE id = ?",
        )
        .bind(user)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}
