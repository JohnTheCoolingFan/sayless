use chrono::Duration;
use serde::{de::Visitor, Deserialize, Deserializer};
use std::sync::Arc;

#[derive(Deserialize, Clone)]
#[serde(rename = "snake_case")]
pub struct IpRecordingConfig {
    #[serde(
        default = "default_retention_period",
        deserialize_with = "deserialize_retention_period"
    )]
    pub retention_period: Duration,
    #[serde(default = "default_check_period")]
    pub retention_check_period: Arc<str>,
}

fn default_check_period() -> Arc<str> {
    "0 0 * * *".into()
}

fn default_retention_period() -> Duration {
    Duration::weeks(2)
}

fn deserialize_retention_period<'de, D: Deserializer<'de>>(des: D) -> Result<Duration, D::Error> {
    struct PeriodVisitor;

    impl<'de> Visitor<'de> for PeriodVisitor {
        type Value = Duration;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("A string signifying a period")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let amount: i64 = v[..(v.len() - 1)]
                .parse()
                .map_err(|e| serde::de::Error::custom(e))?;

            if v.ends_with('Y') {
                Ok(Duration::days(amount * 365))
            } else if v.ends_with('M') {
                Ok(Duration::days(amount * 30))
            } else if v.ends_with('w') {
                Ok(Duration::weeks(amount))
            } else if v.ends_with('d') {
                Ok(Duration::days(amount))
            } else if v.ends_with('h') || v.ends_with('H') {
                Ok(Duration::hours(amount))
            } else if v.ends_with('m') {
                Ok(Duration::minutes(amount))
            } else if v.ends_with('s') {
                Ok(Duration::seconds(amount))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Invalid period suffix: {}",
                    &v[v.len()..]
                )))
            }
        }
    }

    des.deserialize_str(PeriodVisitor)
}
