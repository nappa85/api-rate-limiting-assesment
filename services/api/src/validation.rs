use std::{fmt::Debug, future::Future, pin::Pin};

use axum::{
    extract::{rejection::JsonRejection as AxumJsonRejection, FromRequest},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json as AxumJson,
};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};

// Axum categorizes InvalidData ad error 422, tests needs error 400
// We can discuss as much as we want here, changing tests seems like cheating
// In a real scenario things would probably be handled different
pub struct Json<T>(pub AxumJson<T>);

pub struct JsonRejection(AxumJsonRejection);

impl IntoResponse for JsonRejection {
    fn into_response(self) -> Response {
        if let AxumJsonRejection::JsonDataError(err) = &self.0 {
            if let Some((_, suffix)) = err.body_text().split_once(": validation error: ") {
                return (StatusCode::BAD_REQUEST, suffix.to_owned()).into_response();
            }
        }

        self.0.into_response()
    }
}

impl<T, S> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = JsonRejection;

    fn from_request<'life0, 'async_trait>(
        req: axum::extract::Request,
        state: &'life0 S,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            AxumJson::from_request(req, state)
                .await
                .map(Json)
                .map_err(JsonRejection)
        })
    }
}

pub struct StrLen<const MIN: usize, const MAX: usize, T>(pub T);

impl<const MIN: usize, const MAX: usize, T> Debug for StrLen<MIN, MAX, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, const MIN: usize, const MAX: usize, T> Deserialize<'de> for StrLen<MIN, MAX, T>
where
    T: Deserialize<'de> + AsRef<str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        let str_value = value.as_ref();
        match str_value.chars().count() {
            len if len < MIN => Err(serde::de::Error::custom(format!(
                "validation error: Value too short, min length is {MIN}, got {len}"
            ))),
            len if len > MAX => Err(serde::de::Error::custom(format!(
                "validation error: Value too long, max length is {MAX}, got {len}"
            ))),
            _ => Ok(StrLen(value)),
        }
    }
}

// we can't have const generics of generic type, we could use a macro
// to generate this struct for every numeric typebut it's out of scope
pub struct MinMaxI32<const MIN: i32, const MAX: i32>(pub i32);

impl<const MIN: i32, const MAX: i32> Debug for MinMaxI32<MIN, MAX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, const MIN: i32, const MAX: i32> Deserialize<'de> for MinMaxI32<MIN, MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match i32::deserialize(deserializer)? {
            value if value < MIN => Err(serde::de::Error::custom(format!(
                "validation error: Value too low, min value is {MIN}, got {value}"
            ))),
            value if value > MAX => Err(serde::de::Error::custom(format!(
                "validation error: Value too high, max value is {MAX}, got {value}"
            ))),
            value => Ok(MinMaxI32(value)),
        }
    }
}

// this could probably be generalized as T: Serialize
// but we would be paying a double round-trip of deserialization-serialization
// and it isn't worth it
pub struct NonNullJsonMaxSize<const MAX: usize>(pub serde_json::Value);

impl<const MAX: usize> Debug for NonNullJsonMaxSize<MAX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, const MAX: usize> Deserialize<'de> for NonNullJsonMaxSize<MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_null() {
            Err(serde::de::Error::custom(
                "validation error: Value must not be null",
            ))
        } else {
            // a bit expensive, but can't find a way to avoid allocation
            let len = value.to_string().len();
            if len > MAX {
                Err(serde::de::Error::custom(format!(
                    "validation error: Value too big, max size is {MAX} bytes, got {len} bytes"
                )))
            } else {
                Ok(NonNullJsonMaxSize(value))
            }
        }
    }
}
