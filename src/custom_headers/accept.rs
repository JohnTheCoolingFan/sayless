use axum::http::{HeaderName, HeaderValue};
use headers::Header;
use mime::Mime;

#[derive(Debug, Clone, PartialEq)]
pub struct Accept(pub(crate) Mime);

impl Header for Accept {
    fn name() -> &'static HeaderName {
        &axum::http::header::ACCEPT
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        values
            .next()
            .and_then(|v| v.to_str().ok()?.parse().ok())
            .map(Accept)
            .ok_or_else(headers::Error::invalid)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value = self
            .0
            .as_ref()
            .parse()
            .expect("Mime is always a valid HeaderValue");
        values.extend(std::iter::once(value));
    }
}
