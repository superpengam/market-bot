use std::fmt;

use chrono::{DateTime, Utc};
use market_bot_shared::{DigitalAssetId, FulfillmentId, OrderId, ProductId};
use serde::{Deserialize, Serialize};

use crate::errors::FulfillmentError;
use crate::ports::object_storage::DownloadUrl;

/// Repeating XOR key used only for the reversible sandbox obfuscation.
///
/// Safety: production delivery must replace this with a real envelope cipher
/// and a managed key. The `v1:` prefix lets a future adapter reject unknown
/// versions instead of treating ciphertext as plaintext.
const SANDBOX_XOR_KEY: &[u8] = b"mb.fulfillment.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DigitalAssetType {
    File,
    CardSecret,
    RedeemCode,
}

impl DigitalAssetType {
    pub const fn is_one_time_credential(self) -> bool {
        matches!(self, Self::CardSecret | Self::RedeemCode)
    }
}

/// A digital file, card secret, or redeem code owned by a product.
///
/// Safety: plaintext secrets are never stored on this type. Callers that need
/// the original value must go through [`DigitalAsset::reveal_secret`].
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitalAsset {
    id: DigitalAssetId,
    product_id: ProductId,
    asset_type: DigitalAssetType,
    encrypted_reference: String,
    assigned_order_id: Option<OrderId>,
}

impl DigitalAsset {
    pub fn file(product_id: ProductId, object_key: impl Into<String>) -> Self {
        Self::new(
            product_id,
            DigitalAssetType::File,
            obfuscate(&object_key.into()),
        )
    }

    pub fn card_secret(product_id: ProductId, secret: impl Into<String>) -> Self {
        Self::new(
            product_id,
            DigitalAssetType::CardSecret,
            obfuscate(&secret.into()),
        )
    }

    pub fn redeem_code(product_id: ProductId, code: impl Into<String>) -> Self {
        Self::new(
            product_id,
            DigitalAssetType::RedeemCode,
            obfuscate(&code.into()),
        )
    }

    fn new(
        product_id: ProductId,
        asset_type: DigitalAssetType,
        encrypted_reference: String,
    ) -> Self {
        Self {
            id: DigitalAssetId::new(),
            product_id,
            asset_type,
            encrypted_reference,
            assigned_order_id: None,
        }
    }

    pub const fn id(&self) -> DigitalAssetId {
        self.id
    }

    pub const fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub const fn asset_type(&self) -> DigitalAssetType {
        self.asset_type
    }

    pub fn encrypted_reference(&self) -> &str {
        &self.encrypted_reference
    }

    pub const fn assigned_order_id(&self) -> Option<OrderId> {
        self.assigned_order_id
    }

    /// Reveals the sandbox-obfuscated payload.
    ///
    /// Safety: the returned string is a one-time credential or private object
    /// key. Do not write it to logs, `Display`, or `Debug`.
    pub fn reveal_secret(&self) -> Result<String, FulfillmentError> {
        reveal(&self.encrypted_reference)
    }

    /// Invariant: a one-time credential can be owned by at most one order.
    pub(crate) fn assign_to(&mut self, order_id: OrderId) {
        self.assigned_order_id = Some(order_id);
    }
}

impl fmt::Debug for DigitalAsset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DigitalAsset")
            .field("id", &self.id)
            .field("product_id", &self.product_id)
            .field("asset_type", &self.asset_type)
            .field("encrypted_reference", &self.encrypted_reference)
            .field("assigned_order_id", &self.assigned_order_id)
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct DeliveryReceipt {
    fulfillment_id: FulfillmentId,
    order_id: OrderId,
    download_url: Option<DownloadUrl>,
    revealed_secret: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

impl DeliveryReceipt {
    pub(crate) fn file(
        order_id: OrderId,
        download_url: DownloadUrl,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            fulfillment_id: FulfillmentId::new(),
            order_id,
            download_url: Some(download_url),
            revealed_secret: None,
            expires_at: Some(expires_at),
        }
    }

    pub(crate) fn credential(order_id: OrderId, revealed_secret: String) -> Self {
        Self {
            fulfillment_id: FulfillmentId::new(),
            order_id,
            download_url: None,
            revealed_secret: Some(revealed_secret),
            expires_at: None,
        }
    }

    pub const fn fulfillment_id(&self) -> FulfillmentId {
        self.fulfillment_id
    }

    pub const fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub const fn download_url(&self) -> Option<&DownloadUrl> {
        self.download_url.as_ref()
    }

    pub fn revealed_secret(&self) -> Option<&str> {
        self.revealed_secret.as_deref()
    }

    pub const fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}

impl fmt::Debug for DeliveryReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeliveryReceipt")
            .field("fulfillment_id", &self.fulfillment_id)
            .field("order_id", &self.order_id)
            .field("download_url", &self.download_url)
            .field(
                "revealed_secret",
                &self.revealed_secret.as_ref().map(|_| "[redacted]"),
            )
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

pub(crate) fn obfuscate(plaintext: &str) -> String {
    let encoded = plaintext
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ SANDBOX_XOR_KEY[index % SANDBOX_XOR_KEY.len()])
        .collect::<Vec<_>>();
    format!("v1:{}", encode_hex(&encoded))
}

pub(crate) fn reveal(encrypted_reference: &str) -> Result<String, FulfillmentError> {
    let hex_body = encrypted_reference
        .strip_prefix("v1:")
        .ok_or(FulfillmentError::InvalidEncryptedReference)?;
    let encoded = decode_hex(hex_body)?;
    let plain = encoded
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ SANDBOX_XOR_KEY[index % SANDBOX_XOR_KEY.len()])
        .collect::<Vec<_>>();
    String::from_utf8(plain).map_err(|_| FulfillmentError::InvalidEncryptedReference)
}

fn encode_hex(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(TABLE[(byte >> 4) as usize] as char);
        out.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex(input: &str) -> Result<Vec<u8>, FulfillmentError> {
    if !input.len().is_multiple_of(2) {
        return Err(FulfillmentError::InvalidEncryptedReference);
    }

    let raw = input.as_bytes();
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    for pair in raw.as_chunks::<2>().0 {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn hex_value(digit: u8) -> Result<u8, FulfillmentError> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'a'..=b'f' => Ok(digit - b'a' + 10),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err(FulfillmentError::InvalidEncryptedReference),
    }
}
