//! Pending e-mail address changes of `PATCH /api/v1/user/me/`.
//!
//! A new address is only written once its owner opened the link sent to it. The request lives in
//! Redis for [`EMAIL_CHANGE_TTL_SECONDS`]: `<token>/email_change_request` holds
//! `<user id>|<new address>` and `<user id>/email_change_token` holds the token, so a user has at
//! most one pending request (a new one replaces it) and `GET /user/me/` can show it.
//! `<token>/email_change_spent` marks a token as used.

use crate::redis_keys::{set_token, EMAIL_CHANGE_TTL_SECONDS};
use mairie360_api_lib::redis::error::RedisError;
use mairie360_api_lib::redis::redis_interface::Redis;
use uuid::Uuid;

fn request_key(token: &str) -> String {
    format!("{token}/email_change_request")
}

fn spent_key(token: &str) -> String {
    format!("{token}/email_change_spent")
}

fn user_key(user_id: u64) -> String {
    format!("{user_id}/email_change_token")
}

/// Removes the pending request of a user, if any.
///
/// # Errors
///
/// A [`RedisError`] when Redis refuses a command.
pub async fn cancel(redis: &Redis, user_id: u64) -> Result<(), RedisError> {
    if let Some(token) = redis.secure_get::<String>(&user_key(user_id)).await? {
        redis.secure_delete(&request_key(&token)).await?;
    }
    redis.secure_delete(&user_key(user_id)).await
}

/// A fresh single-use token for a request, not valid until [`publish`] records it.
#[must_use]
pub fn new_token() -> String {
    Uuid::new_v4().to_string()
}

/// Records the request `token` to move `user_id` to `new_email`, replacing the previous one.
///
/// Call it once the confirmation e-mail was delivered: a failed delivery then leaves the request
/// already sent untouched. The token of the user is overwritten in one command, so the replaced
/// token stops being accepted at once ([`take`] checks it is the current one); the old request
/// is only cleaned up afterwards.
///
/// # Errors
///
/// A [`RedisError`] when Redis refuses a command.
pub async fn publish(
    redis: &Redis,
    user_id: u64,
    new_email: &str,
    token: &str,
) -> Result<(), RedisError> {
    let previous = redis.secure_get::<String>(&user_key(user_id)).await?;
    set_token(
        redis,
        &request_key(token),
        &format!("{user_id}|{new_email}"),
        EMAIL_CHANGE_TTL_SECONDS,
    )
    .await?;
    redis
        .set_ex(&user_key(user_id), token, EMAIL_CHANGE_TTL_SECONDS)
        .await?;
    if let Some(previous) = previous.filter(|previous| previous != token) {
        redis.secure_delete(&request_key(&previous)).await?;
    }
    Ok(())
}

/// The address a user is waiting to confirm, `None` when there is no pending request.
///
/// # Errors
///
/// A [`RedisError`] when Redis refuses a command.
pub async fn pending_email(redis: &Redis, user_id: u64) -> Result<Option<String>, RedisError> {
    let Some(token) = redis.secure_get::<String>(&user_key(user_id)).await? else {
        return Ok(None);
    };
    Ok(redis
        .secure_get::<String>(&request_key(&token))
        .await?
        .and_then(|value| parse(&value).map(|(_, email)| email)))
}

/// Spends a token: the request it names (user id and new address) is returned once, then gone.
///
/// A token that is not the current one of its user (replaced by a newer request or cancelled) is
/// refused, and the spending is one `SET NX` on a marker key, so two concurrent confirmations
/// cannot both win.
///
/// # Errors
///
/// A [`RedisError`] when Redis refuses a command.
pub async fn take(redis: &Redis, token: &str) -> Result<Option<(u64, String)>, RedisError> {
    let Some(value) = redis.secure_get::<String>(&request_key(token)).await? else {
        return Ok(None);
    };
    let Some((user_id, email)) = parse(&value) else {
        return Ok(None);
    };
    if redis
        .secure_get::<String>(&user_key(user_id))
        .await?
        .as_deref()
        != Some(token)
    {
        return Ok(None);
    }
    if !redis
        .secure_set_ex(&spent_key(token), "1", EMAIL_CHANGE_TTL_SECONDS)
        .await?
    {
        return Ok(None);
    }
    redis.secure_delete(&request_key(token)).await?;
    redis.secure_delete(&user_key(user_id)).await?;
    Ok(Some((user_id, email)))
}

fn parse(value: &str) -> Option<(u64, String)> {
    let (user_id, email) = value.split_once('|')?;
    Some((user_id.parse().ok()?, email.to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_a_request() {
        assert_eq!(
            parse("42|jean@mairie360.fr"),
            Some((42, "jean@mairie360.fr".to_string()))
        );
    }

    #[test]
    fn refuses_a_malformed_request() {
        assert_eq!(parse("jean@mairie360.fr"), None);
        assert_eq!(parse("x|jean@mairie360.fr"), None);
    }
}
