use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::{
    domain::payment::PaymentEvent,
    ports::payment_provider::{
        PaymentIntent, PaymentIntentInput, PaymentProvider, ProviderError, RefundIntent,
        RefundIntentInput, SANDBOX_SIGNATURE_HEADER, SettlementIntent, SettlementReleaseInput,
        VerifiedPaymentEvent, WebhookHeaders,
    },
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
pub struct SandboxPaymentProvider {
    webhook_secret: Vec<u8>,
}

impl SandboxPaymentProvider {
    pub fn new(webhook_secret: impl AsRef<[u8]>) -> Self {
        Self {
            webhook_secret: webhook_secret.as_ref().to_vec(),
        }
    }

    pub fn sign_webhook(&self, payload: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.webhook_secret)
            .expect("HMAC accepts keys of any length");
        mac.update(payload);
        hex_encode(&mac.finalize().into_bytes())
    }

    pub fn signed_headers(&self, payload: &[u8]) -> WebhookHeaders {
        let mut headers = WebhookHeaders::new();
        headers.insert(SANDBOX_SIGNATURE_HEADER, self.sign_webhook(payload));
        headers
    }
}

#[async_trait]
impl PaymentProvider for SandboxPaymentProvider {
    async fn create_payment_intent(
        &self,
        input: PaymentIntentInput,
    ) -> Result<PaymentIntent, ProviderError> {
        Ok(PaymentIntent::new(
            format!("sandbox_{}", input.order_id),
            input.order_id,
            input.amount,
        ))
    }

    fn verify_webhook(
        &self,
        headers: &WebhookHeaders,
        body: &[u8],
    ) -> Result<VerifiedPaymentEvent, ProviderError> {
        let signature = headers
            .get(SANDBOX_SIGNATURE_HEADER)
            .ok_or(ProviderError::InvalidSignature)?;
        let expected = self.sign_webhook(body);
        if expected.as_bytes().ct_eq(signature.as_bytes()).unwrap_u8() != 1 {
            return Err(ProviderError::InvalidSignature);
        }

        let event: PaymentEvent =
            serde_json::from_slice(body).map_err(|_| ProviderError::InvalidPayload)?;
        Ok(VerifiedPaymentEvent::from_verified(event))
    }

    async fn create_refund(&self, input: RefundIntentInput) -> Result<RefundIntent, ProviderError> {
        Ok(RefundIntent::new(
            format!("sandbox_refund_{}", input.refund_id.as_uuid()),
            input.amount,
        ))
    }

    async fn release_settlement(
        &self,
        input: SettlementReleaseInput,
    ) -> Result<SettlementIntent, ProviderError> {
        Ok(SettlementIntent::new(
            format!("sandbox_settlement_{}", input.settlement_id.as_uuid()),
            input.amount,
        ))
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
