use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestContext {
    request_id: Uuid,
    idempotency_key: Option<String>,
}

impl RequestContext {
    pub fn new(
        request_id: Uuid,
        idempotency_key: Option<String>,
    ) -> Result<Self, RequestContextError> {
        let idempotency_key = idempotency_key.map(|key| key.trim().to_owned());
        if idempotency_key.as_deref().is_some_and(str::is_empty) {
            return Err(RequestContextError::BlankIdempotencyKey);
        }

        Ok(Self {
            request_id,
            idempotency_key,
        })
    }

    pub const fn request_id(&self) -> Uuid {
        self.request_id
    }

    pub fn idempotency_key(&self) -> Option<&str> {
        self.idempotency_key.as_deref()
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum RequestContextError {
    #[error("idempotency key cannot be blank")]
    BlankIdempotencyKey,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RequestMetadata {
    pub request_id: Uuid,
    pub idempotency_key: Option<String>,
}

impl From<&RequestContext> for RequestMetadata {
    fn from(context: &RequestContext) -> Self {
        Self {
            request_id: context.request_id(),
            idempotency_key: context.idempotency_key().map(str::to_owned),
        }
    }
}
