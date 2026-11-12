//! Directed UI requests. Call ids are scoped to a control connection, never replayed.
use super::{Client, Shared};
use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::time::{Duration, Instant};

pub const MAX_WAIT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Deserialize)]
pub struct Request {
    pub call_id: String,
    pub action: String,
    #[serde(default)]
    pub args: Value,
    pub session_id: Option<String>,
    pub block_id: Option<String>,
    #[serde(default)]
    pub requester: Requester,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Requester {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub info: Value,
    #[serde(default)]
    pub principal: Value,
}
impl std::fmt::Display for Requester {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name: String = self.info["name"]
            .as_str()
            .unwrap_or(&self.client_id)
            .chars()
            .filter(|c| {
                !c.is_control()
                    && !matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
            .take(40)
            .collect();
        let principal = self.principal["display"]
            .as_str()
            .or_else(|| self.principal["login_name"].as_str())
            .or_else(|| self.principal["kind"].as_str())
            .unwrap_or("unknown principal");
        write!(f, "{name} ({principal})")
    }
}

#[derive(Clone, Debug)]
pub struct Pending {
    pub epoch: u64,
    pub received: Instant,
    pub request: Request,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Performed,
    Error { code: &'static str, message: String },
}
impl Outcome {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self::Error {
            code,
            message: message.into(),
        }
    }
    pub fn reply(&self, call_id: &str) -> rexrs::api::client::Reply {
        let reply = rexrs::api::client::Reply::new(call_id);
        match self {
            Self::Performed => reply.result(Value::Null),
            Self::Error { code, message } => {
                reply.error(rexrs::api::client::ReplyError::new(*code, message))
            }
        }
    }
}

pub(super) async fn reply(control: &mut Client, call_id: &str, outcome: &Outcome) -> Result<()> {
    match tokio::time::timeout(Duration::from_secs(8), control.call(outcome.reply(call_id))).await {
        Ok(Ok(_)) => {}
        Ok(Err(
            error @ (rexrs::Error::Transport(_)
            | rexrs::Error::Closed { .. }
            | rexrs::Error::Protocol(_)),
        )) => return Err(error.into()),
        Ok(Err(error)) => log::debug!("client.reply failed (not retried): {error}"),
        Err(error) => log::debug!("client.reply timed out (not retried): {error}"),
    }
    Ok(())
}

pub(super) async fn receive(
    control: &mut Client,
    shared: &Shared,
    payload: Value,
    busy_since: Option<Instant>,
) -> Result<()> {
    let received = busy_since.unwrap_or_else(Instant::now);
    let Some(call_id) = payload["call_id"].as_str().map(str::to_owned) else {
        return Ok(());
    };
    let request = match serde_json::from_value::<Request>(payload) {
        Ok(request) => request,
        Err(_) => {
            return reply(
                control,
                &call_id,
                &Outcome::error("bad_request", "Malformed remote request; nothing was done."),
            )
            .await;
        }
    };
    if received.elapsed() >= MAX_WAIT {
        return reply(
            control,
            &call_id,
            &Outcome::error("internal", "Rex Rune was busy; nothing was done."),
        )
        .await;
    }
    shared.update(|m| {
        m.performs.push(Pending {
            epoch: m.connection_epoch,
            received,
            request,
        })
    });
    Ok(())
}

pub(super) async fn expire(control: &mut Client, shared: &Shared) -> Result<()> {
    let expired = {
        let mut model = shared.model.lock().expect("host model");
        let (expired, waiting) = std::mem::take(&mut model.performs)
            .into_iter()
            .partition::<Vec<_>, _>(|p| p.received.elapsed() >= MAX_WAIT);
        model.performs = waiting;
        expired
    };
    for pending in expired {
        reply(
            control,
            &pending.request.call_id,
            &Outcome::error(
                "internal",
                "Rex Rune did not act in time; nothing was done.",
            ),
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requester_names_are_bounded_and_cannot_inject_controls_or_bidi() {
        let mut requester = Requester {
            info: serde_json::json!({"name": "re\n\r\t\u{001b}\u{0085}\u{061c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}x"}),
            principal: serde_json::json!({"display": "uid 1000"}),
            ..Requester::default()
        };
        assert_eq!(requester.to_string(), "rex (uid 1000)");
        requester.info = serde_json::json!({"name": "界".repeat(50)});
        assert_eq!(
            requester.to_string(),
            format!("{} (uid 1000)", "界".repeat(40))
        );
        requester.info = Value::Null;
        requester.client_id = "c\nli\u{202e}".into();
        assert_eq!(requester.to_string(), "cli (uid 1000)");
    }

    #[test]
    fn success_is_explicit_null_and_errors_have_no_result() {
        let success = serde_json::to_value(Outcome::Performed.reply("1")).unwrap();
        assert_eq!(success.get("result"), Some(&Value::Null));
        assert!(success.get("error").is_none());
        for code in [
            "refused",
            "not_supported",
            "bad_request",
            "not_found",
            "internal",
        ] {
            let error =
                serde_json::to_value(Outcome::error(code, "nothing done").reply("1")).unwrap();
            assert!(error.get("result").is_none());
            assert_eq!(error["error"]["code"], code);
        }
    }
}
