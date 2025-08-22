use std::str::FromStr;

use diesel::{associations::HasTable, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use postgres_models::{
    models::{NewRateLimit, RateLimit},
    schema, DbConnection,
};
use strum::{AsRefStr, EnumString};
use tracing::warn;

use crate::errors::AppError;

#[derive(Copy, Clone, Debug, Default, EnumString, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum AccountType {
    #[default]
    Basic,
    Premium,
    Enterprise,
}

impl AccountType {
    pub fn max_requests(self) -> i32 {
        match self {
            AccountType::Basic => 20,
            AccountType::Premium => 100,
            AccountType::Enterprise => 500,
        }
    }

    pub fn window_seconds(self) -> i32 {
        60 // all limits are per minute
    }
}

pub async fn get_or_create_rate_limit(
    db_conn: &mut DbConnection,
    account_id: &str,
) -> Result<RateLimit, AppError> {
    // tests are prefixing limit type, normally we shouldn't trust user input
    let limit_type = account_id
        .split_once('_')
        .and_then(|(prefix, _)| {
            AccountType::from_str(prefix)
                .inspect_err(|_| {
                    warn!("Can't decode account type from account_id \"{account_id}\"")
                })
                .ok()
        })
        .unwrap_or_default();

    use schema::rate_limits::dsl::{
        account_id as col_account_id, limit_type as col_limit_type, rate_limits,
    };

    loop {
        if let Some(rate_limit) = rate_limits
            .filter(col_account_id.eq(account_id))
            .filter(col_limit_type.eq(limit_type.as_ref()))
            .first::<RateLimit>(db_conn)
            .await
            .optional()?
        {
            return Ok(rate_limit);
        }

        // in case of race condition, we simply retry
        let new_rate_limit = NewRateLimit::new(
            account_id.to_string(),
            limit_type.as_ref().to_string(),
            limit_type.max_requests(),
            limit_type.window_seconds(),
        );
        if let Some(rate_limit) = diesel::insert_into(rate_limits::table())
            .values(new_rate_limit)
            .on_conflict_do_nothing()
            .get_result::<RateLimit>(db_conn)
            .await
            .optional()?
        {
            return Ok(rate_limit);
        }
    }
}
