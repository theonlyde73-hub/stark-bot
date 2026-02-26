//! x402 payment verification via EIP-712 signature recovery.
//!
//! Verifies incoming x402 payment signatures by:
//! 1. Decoding the base64 X-Payment header into a PaymentPayload
//! 2. Rebuilding the EIP-712 typed data hash (domain + struct hash)
//! 3. Recovering the signer address via ecrecover
//! 4. Checking recovered address, amount, and expiry

use ethers::types::{Address, H256, U256, Signature};
use ethers::utils::keccak256;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::*;

/// Result of verifying an x402 payment signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub payer: String,
    pub amount: String,
    pub currency: String,
    pub scheme: String,
    pub nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl VerificationResult {
    fn invalid(reason: &str) -> Self {
        Self {
            valid: false,
            payer: String::new(),
            amount: String::new(),
            currency: String::new(),
            scheme: String::new(),
            nonce: String::new(),
            expiry: None,
            error: Some(reason.to_string()),
        }
    }
}

/// Requirements the caller specifies for verification.
#[derive(Debug, Clone, Deserialize)]
pub struct VerifyRequirements {
    pub price: String,
    pub currency: String,
    pub payee: String,
    pub network: String,
    /// Token contract address (defaults to USDC on the given network)
    #[serde(default)]
    pub asset: Option<String>,
    /// Token name for EIP-712 domain (defaults to "USD Coin")
    #[serde(default)]
    pub token_name: Option<String>,
    /// Token version for EIP-712 domain (defaults to "2")
    #[serde(default)]
    pub token_version: Option<String>,
    /// Token decimals (defaults to 6)
    #[serde(default)]
    pub decimals: Option<u8>,
}

/// Decode an X-Payment header value into a PaymentPayload.
/// Tries base64 first, then raw JSON.
pub fn decode_payment_header(header: &str) -> Result<serde_json::Value, String> {
    // Try base64 decode first
    if let Ok(decoded) = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        header.trim(),
    ) {
        if let Ok(s) = String::from_utf8(decoded) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                return Ok(v);
            }
        }
    }
    // Try raw JSON
    serde_json::from_str(header)
        .map_err(|e| format!("Failed to decode X-Payment header: {}", e))
}

/// Verify an x402 payment from a decoded payment payload JSON.
pub fn verify_payment(
    payload_json: &serde_json::Value,
    requirements: &VerifyRequirements,
) -> VerificationResult {
    // Extract scheme (V1: top-level, V2: in accepted)
    let scheme = payload_json.get("scheme")
        .or_else(|| payload_json.get("accepted").and_then(|a| a.get("scheme")))
        .and_then(|v| v.as_str())
        .unwrap_or("exact");

    // Extract network
    let network = payload_json.get("network")
        .or_else(|| payload_json.get("accepted").and_then(|a| a.get("network")))
        .and_then(|v| v.as_str())
        .unwrap_or(&requirements.network);

    // Extract payload.signature and payload.authorization
    let payload = match payload_json.get("payload") {
        Some(p) => p,
        None => return VerificationResult::invalid("Missing 'payload' field"),
    };

    let signature_hex = match payload.get("signature").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing 'payload.signature'"),
    };

    let authorization = match payload.get("authorization") {
        Some(a) => a,
        None => return VerificationResult::invalid("Missing 'payload.authorization'"),
    };

    // Build token metadata for EIP-712 domain
    let chain_id = chain_id_for_network(network);
    let token_address_str = requirements.asset.clone()
        .unwrap_or_else(|| USDC_ADDRESS.to_string());
    let token_name = requirements.token_name.clone()
        .unwrap_or_else(|| "USD Coin".to_string());
    let token_version = requirements.token_version.clone()
        .unwrap_or_else(|| "2".to_string());
    let decimals = requirements.decimals.unwrap_or(6);

    let token_address: Address = match token_address_str.parse() {
        Ok(a) => a,
        Err(_) => return VerificationResult::invalid("Invalid token address"),
    };

    // Build EIP-712 domain separator
    let domain_separator = compute_domain_separator(
        &token_name,
        &token_version,
        chain_id,
        token_address,
    );

    match scheme {
        "exact" | "eip3009" => verify_eip3009(
            authorization, signature_hex, &domain_separator,
            requirements, decimals,
        ),
        "permit" => verify_eip2612(
            authorization, signature_hex, &domain_separator,
            requirements, decimals,
        ),
        other => VerificationResult::invalid(&format!("Unsupported scheme: {}", other)),
    }
}

/// Compute EIP-712 domain separator hash.
fn compute_domain_separator(
    name: &str,
    version: &str,
    chain_id: u64,
    verifying_contract: Address,
) -> H256 {
    let type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
    );
    let name_hash = keccak256(name.as_bytes());
    let version_hash = keccak256(version.as_bytes());

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&name_hash);
    encoded.extend_from_slice(&version_hash);
    encoded.extend_from_slice(&ethers::abi::encode(&[
        ethers::abi::Token::Uint(U256::from(chain_id)),
    ]));
    encoded.extend_from_slice(&ethers::abi::encode(&[
        ethers::abi::Token::Address(verifying_contract),
    ]));

    H256::from(keccak256(&encoded))
}

/// Verify EIP-3009 TransferWithAuthorization signature.
fn verify_eip3009(
    authorization: &serde_json::Value,
    signature_hex: &str,
    domain_separator: &H256,
    requirements: &VerifyRequirements,
    decimals: u8,
) -> VerificationResult {
    // Parse authorization fields
    let from_str = match authorization.get("from").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.from"),
    };
    let to_str = match authorization.get("to").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.to"),
    };
    let value_str = match authorization.get("value").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.value"),
    };
    let valid_before_str = match authorization.get("validBefore").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.validBefore"),
    };
    let valid_after_str = authorization.get("validAfter")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let nonce_str = match authorization.get("nonce").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.nonce"),
    };

    // Parse addresses
    let from: Address = match from_str.parse() {
        Ok(a) => a,
        Err(_) => return VerificationResult::invalid("Invalid from address"),
    };
    let to: Address = match to_str.parse() {
        Ok(a) => a,
        Err(_) => return VerificationResult::invalid("Invalid to address"),
    };

    // Parse value
    let value = match U256::from_dec_str(value_str) {
        Ok(v) => v,
        Err(_) => return VerificationResult::invalid("Invalid value"),
    };

    // Parse valid_before and check expiry
    let valid_before = match U256::from_dec_str(valid_before_str) {
        Ok(v) => v,
        Err(_) => return VerificationResult::invalid("Invalid validBefore"),
    };
    let valid_after = U256::from_dec_str(valid_after_str).unwrap_or(U256::zero());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if valid_before <= U256::from(now) {
        return VerificationResult::invalid("Payment has expired (validBefore <= now)");
    }
    if valid_after > U256::from(now) {
        return VerificationResult::invalid("Payment not yet valid (validAfter > now)");
    }

    // Check payee matches
    let payee: Address = match requirements.payee.parse() {
        Ok(a) => a,
        Err(_) => return VerificationResult::invalid("Invalid payee address in requirements"),
    };
    if to != payee {
        return VerificationResult::invalid(&format!(
            "Payment 'to' ({:?}) does not match required payee ({:?})", to, payee
        ));
    }

    // Check amount >= required price
    let required_amount = match parse_token_amount(&requirements.price, decimals) {
        Ok(v) => v,
        Err(e) => return VerificationResult::invalid(&format!("Invalid price: {}", e)),
    };
    if value < required_amount {
        return VerificationResult::invalid(&format!(
            "Payment amount {} < required {}", value, required_amount
        ));
    }

    // Parse nonce as bytes32
    let nonce: H256 = match nonce_str.parse() {
        Ok(n) => n,
        Err(_) => return VerificationResult::invalid("Invalid nonce (expected bytes32 hex)"),
    };

    // Build struct hash for TransferWithAuthorization
    let type_hash = keccak256(
        b"TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
    );
    let struct_hash = H256::from(keccak256(&ethers::abi::encode(&[
        ethers::abi::Token::FixedBytes(type_hash.to_vec()),
        ethers::abi::Token::Address(from),
        ethers::abi::Token::Address(to),
        ethers::abi::Token::Uint(value),
        ethers::abi::Token::Uint(valid_after),
        ethers::abi::Token::Uint(valid_before),
        ethers::abi::Token::FixedBytes(nonce.as_bytes().to_vec()),
    ])));

    // Build EIP-712 digest: 0x19 0x01 ++ domainSeparator ++ structHash
    let mut to_sign = Vec::with_capacity(66);
    to_sign.push(0x19);
    to_sign.push(0x01);
    to_sign.extend_from_slice(domain_separator.as_bytes());
    to_sign.extend_from_slice(struct_hash.as_bytes());
    let digest = H256::from(keccak256(&to_sign));

    // Recover signer from signature
    let recovered = match recover_signer(digest, signature_hex) {
        Ok(addr) => addr,
        Err(e) => return VerificationResult::invalid(&format!("Signature recovery failed: {}", e)),
    };

    // Verify recovered address matches from
    if recovered != from {
        return VerificationResult::invalid(&format!(
            "Recovered signer {:?} does not match from {:?}", recovered, from
        ));
    }

    VerificationResult {
        valid: true,
        payer: format!("{:?}", from),
        amount: value_str.to_string(),
        currency: requirements.currency.clone(),
        scheme: "exact".to_string(),
        nonce: nonce_str.to_string(),
        expiry: valid_before.try_into().ok(),
        error: None,
    }
}

/// Verify EIP-2612 Permit signature.
fn verify_eip2612(
    authorization: &serde_json::Value,
    signature_hex: &str,
    domain_separator: &H256,
    requirements: &VerifyRequirements,
    decimals: u8,
) -> VerificationResult {
    let owner_str = match authorization.get("owner").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.owner"),
    };
    let spender_str = match authorization.get("spender").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.spender"),
    };
    let value_str = match authorization.get("value").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.value"),
    };
    let nonce_str = match authorization.get("nonce").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.nonce"),
    };
    let deadline_str = match authorization.get("deadline").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing authorization.deadline"),
    };

    let owner: Address = match owner_str.parse() {
        Ok(a) => a,
        Err(_) => return VerificationResult::invalid("Invalid owner address"),
    };
    let spender: Address = match spender_str.parse() {
        Ok(a) => a,
        Err(_) => return VerificationResult::invalid("Invalid spender address"),
    };
    let value = match U256::from_dec_str(value_str) {
        Ok(v) => v,
        Err(_) => return VerificationResult::invalid("Invalid value"),
    };
    let nonce = match U256::from_dec_str(nonce_str) {
        Ok(v) => v,
        Err(_) => return VerificationResult::invalid("Invalid nonce"),
    };
    let deadline = match U256::from_dec_str(deadline_str) {
        Ok(v) => v,
        Err(_) => return VerificationResult::invalid("Invalid deadline"),
    };

    // Check deadline
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if deadline <= U256::from(now) {
        return VerificationResult::invalid("Permit has expired (deadline <= now)");
    }

    // Check amount >= required price
    let required_amount = match parse_token_amount(&requirements.price, decimals) {
        Ok(v) => v,
        Err(e) => return VerificationResult::invalid(&format!("Invalid price: {}", e)),
    };
    if value < required_amount {
        return VerificationResult::invalid(&format!(
            "Payment amount {} < required {}", value, required_amount
        ));
    }

    // Build struct hash for Permit
    let type_hash = keccak256(
        b"Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)"
    );
    let struct_hash = H256::from(keccak256(&ethers::abi::encode(&[
        ethers::abi::Token::FixedBytes(type_hash.to_vec()),
        ethers::abi::Token::Address(owner),
        ethers::abi::Token::Address(spender),
        ethers::abi::Token::Uint(value),
        ethers::abi::Token::Uint(nonce),
        ethers::abi::Token::Uint(deadline),
    ])));

    // Build EIP-712 digest
    let mut to_sign = Vec::with_capacity(66);
    to_sign.push(0x19);
    to_sign.push(0x01);
    to_sign.extend_from_slice(domain_separator.as_bytes());
    to_sign.extend_from_slice(struct_hash.as_bytes());
    let digest = H256::from(keccak256(&to_sign));

    // Recover signer
    let recovered = match recover_signer(digest, signature_hex) {
        Ok(addr) => addr,
        Err(e) => return VerificationResult::invalid(&format!("Signature recovery failed: {}", e)),
    };

    // Verify recovered address matches owner
    if recovered != owner {
        return VerificationResult::invalid(&format!(
            "Recovered signer {:?} does not match owner {:?}", recovered, owner
        ));
    }

    VerificationResult {
        valid: true,
        payer: format!("{:?}", owner),
        amount: value_str.to_string(),
        currency: requirements.currency.clone(),
        scheme: "permit".to_string(),
        nonce: nonce_str.to_string(),
        expiry: deadline.try_into().ok(),
        error: None,
    }
}

/// Recover signer address from an EIP-712 digest and hex-encoded signature.
fn recover_signer(digest: H256, signature_hex: &str) -> Result<Address, String> {
    let sig_bytes = hex::decode(signature_hex.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid signature hex: {}", e))?;

    if sig_bytes.len() != 65 {
        return Err(format!("Signature must be 65 bytes, got {}", sig_bytes.len()));
    }

    // Parse r, s, v from the 65-byte signature
    let r = H256::from_slice(&sig_bytes[0..32]);
    let s = H256::from_slice(&sig_bytes[32..64]);
    let v = sig_bytes[64];

    // Normalize v: Ethereum uses 27/28, but some libraries use 0/1
    let recovery_id = if v >= 27 { v - 27 } else { v };

    let signature = Signature {
        r: U256::from_big_endian(r.as_bytes()),
        s: U256::from_big_endian(s.as_bytes()),
        v: recovery_id as u64,
    };

    signature.recover(digest)
        .map_err(|e| format!("ecrecover failed: {}", e))
}

/// Parse a human-readable token amount (e.g. "0.01") into the smallest unit.
/// For USDC with 6 decimals: "0.01" → 10000
pub fn parse_token_amount(amount: &str, decimals: u8) -> Result<U256, String> {
    // If it looks like a raw integer already (no decimal point, large number), use as-is
    if !amount.contains('.') {
        return U256::from_dec_str(amount)
            .map_err(|e| format!("Invalid amount '{}': {}", amount, e));
    }

    let parts: Vec<&str> = amount.split('.').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid decimal amount: {}", amount));
    }

    let whole: U256 = if parts[0].is_empty() {
        U256::zero()
    } else {
        U256::from_dec_str(parts[0])
            .map_err(|e| format!("Invalid whole part: {}", e))?
    };

    let frac_str = parts[1];
    let frac_len = frac_str.len();

    if frac_len > decimals as usize {
        return Err(format!(
            "Too many decimal places ({}) for {} decimals", frac_len, decimals
        ));
    }

    // Pad fractional part to full decimals
    let padded = format!("{:0<width$}", frac_str, width = decimals as usize);
    let frac: U256 = U256::from_dec_str(&padded)
        .map_err(|e| format!("Invalid fractional part: {}", e))?;

    let multiplier = U256::from(10u64).pow(U256::from(decimals));
    Ok(whole * multiplier + frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_amount() {
        // USDC 6 decimals
        assert_eq!(parse_token_amount("0.01", 6).unwrap(), U256::from(10000u64));
        assert_eq!(parse_token_amount("1.0", 6).unwrap(), U256::from(1000000u64));
        assert_eq!(parse_token_amount("0.000001", 6).unwrap(), U256::from(1u64));
        assert_eq!(parse_token_amount("10000", 6).unwrap(), U256::from(10000u64)); // raw integer
    }

    #[test]
    fn test_domain_separator() {
        // Smoke test: domain separator should be 32 bytes
        let sep = compute_domain_separator(
            "USD Coin",
            "2",
            8453,
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".parse().unwrap(),
        );
        assert_ne!(sep, H256::zero());
    }

    #[test]
    fn test_verification_result_invalid() {
        let r = VerificationResult::invalid("test error");
        assert!(!r.valid);
        assert_eq!(r.error.as_deref(), Some("test error"));
    }
}
