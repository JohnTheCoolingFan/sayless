use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
pub struct CreateTokenParams {
    #[serde(default)]
    pub admin_perm: bool,
    #[serde(default)]
    pub create_link_perm: bool,
    #[serde(default)]
    pub view_ips_perm: bool,
    #[serde(default, deserialize_with = "deser_timestamp")]
    pub expires_at: Option<DateTime<Utc>>,
}

fn deser_timestamp<'de, D: Deserializer<'de>>(des: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    Ok(Some(
        NaiveDateTime::parse_from_str(&String::deserialize(des)?, "%Y-%m-%d %H:%M:%S%.f")
            .map_err(serde::de::Error::custom)?
            .and_utc(),
    ))
}
