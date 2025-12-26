//! X.509 certificate secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use async_trait::async_trait;
use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::{X509, X509Builder, X509NameBuilder, X509Extension};
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectAlternativeName};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{Utc, Duration};

/// X.509 certificate types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CertType {
    /// Certificate Authority
    CA,
    /// Self-signed certificate
    #[serde(rename = "self-signed")]
    SelfSigned,
    /// Certificate signed by a CA
    Signed,
}

/// X.509 certificate secret.
#[derive(Debug, Clone)]
pub struct X509Secret {
    path: String,
    cert_type: CertType,
    common_name: String,
    organization: Option<String>,
    organizational_unit: Option<String>,
    country: Option<String>,
    state: Option<String>,
    locality: Option<String>,
    alternate_names: Vec<String>,
    key_size: u32,
    validity_days: i64,
    ca_path: Option<String>,
    is_server_cert: bool,
    is_client_cert: bool,
}

impl X509Secret {
    /// Create from definition hashmap.
    pub fn from_definition(path: String, mut def: HashMap<String, serde_json::Value>) -> Result<Self> {
        let cert_type = def.remove("cert_type")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(CertType::Signed);

        let common_name = def.remove("common_name")
            .or_else(|| def.remove("cn"))
            .and_then(|v| v.as_str().map(String::from))
            .ok_or_else(|| GenesisError::Secret("Missing common_name for X509 certificate".to_string()))?;

        let organization = def.remove("organization")
            .or_else(|| def.remove("o"))
            .and_then(|v| v.as_str().map(String::from));

        let organizational_unit = def.remove("organizational_unit")
            .or_else(|| def.remove("ou"))
            .and_then(|v| v.as_str().map(String::from));

        let country = def.remove("country")
            .or_else(|| def.remove("c"))
            .and_then(|v| v.as_str().map(String::from));

        let state = def.remove("state")
            .or_else(|| def.remove("st"))
            .and_then(|v| v.as_str().map(String::from));

        let locality = def.remove("locality")
            .or_else(|| def.remove("l"))
            .and_then(|v| v.as_str().map(String::from));

        let alternate_names = def.remove("alternate_names")
            .or_else(|| def.remove("alt_names"))
            .and_then(|v| {
                if let Some(arr) = v.as_array() {
                    Some(arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let key_size = def.remove("key_size")
            .or_else(|| def.remove("bits"))
            .and_then(|v| v.as_u64().map(|n| n as u32))
            .unwrap_or(2048);

        let validity_days = def.remove("validity")
            .or_else(|| def.remove("valid_for"))
            .and_then(|v| v.as_i64())
            .unwrap_or(365);

        let ca_path = def.remove("signed_by")
            .or_else(|| def.remove("ca"))
            .and_then(|v| v.as_str().map(String::from));

        let is_server_cert = def.remove("usage")
            .and_then(|v| v.as_str())
            .map(|s| s.contains("server"))
            .unwrap_or(true);

        let is_client_cert = def.remove("usage")
            .and_then(|v| v.as_str())
            .map(|s| s.contains("client"))
            .unwrap_or(false);

        Ok(Self {
            path,
            cert_type,
            common_name,
            organization,
            organizational_unit,
            country,
            state,
            locality,
            alternate_names,
            key_size,
            validity_days,
            ca_path,
            is_server_cert,
            is_client_cert,
        })
    }

    fn generate_private_key(&self) -> Result<PKey<Private>> {
        let rsa = Rsa::generate(self.key_size)
            .map_err(|e| GenesisError::Secret(format!("Failed to generate RSA key: {}", e)))?;

        PKey::from_rsa(rsa)
            .map_err(|e| GenesisError::Secret(format!("Failed to create private key: {}", e)))
    }

    fn build_name(&self) -> Result<openssl::x509::X509Name> {
        let mut builder = X509NameBuilder::new()
            .map_err(|e| GenesisError::Secret(format!("Failed to create name builder: {}", e)))?;

        builder.append_entry_by_text("CN", &self.common_name)
            .map_err(|e| GenesisError::Secret(format!("Failed to set CN: {}", e)))?;

        if let Some(ref o) = self.organization {
            builder.append_entry_by_text("O", o)
                .map_err(|e| GenesisError::Secret(format!("Failed to set O: {}", e)))?;
        }

        if let Some(ref ou) = self.organizational_unit {
            builder.append_entry_by_text("OU", ou)
                .map_err(|e| GenesisError::Secret(format!("Failed to set OU: {}", e)))?;
        }

        if let Some(ref c) = self.country {
            builder.append_entry_by_text("C", c)
                .map_err(|e| GenesisError::Secret(format!("Failed to set C: {}", e)))?;
        }

        if let Some(ref st) = self.state {
            builder.append_entry_by_text("ST", st)
                .map_err(|e| GenesisError::Secret(format!("Failed to set ST: {}", e)))?;
        }

        if let Some(ref l) = self.locality {
            builder.append_entry_by_text("L", l)
                .map_err(|e| GenesisError::Secret(format!("Failed to set L: {}", e)))?;
        }

        Ok(builder.build())
    }

    fn generate_ca(&self, key: &PKey<Private>) -> Result<X509> {
        let mut builder = X509Builder::new()
            .map_err(|e| GenesisError::Secret(format!("Failed to create X509 builder: {}", e)))?;

        builder.set_version(2)
            .map_err(|e| GenesisError::Secret(format!("Failed to set version: {}", e)))?;

        let serial = BigNum::from_u32(1)
            .map_err(|e| GenesisError::Secret(format!("Failed to create serial: {}", e)))?;
        builder.set_serial_number(&serial.to_asn1_integer()
            .map_err(|e| GenesisError::Secret(format!("Failed to set serial: {}", e)))?)
            .map_err(|e| GenesisError::Secret(format!("Failed to set serial number: {}", e)))?;

        let name = self.build_name()?;
        builder.set_subject_name(&name)
            .map_err(|e| GenesisError::Secret(format!("Failed to set subject: {}", e)))?;
        builder.set_issuer_name(&name)
            .map_err(|e| GenesisError::Secret(format!("Failed to set issuer: {}", e)))?;

        let not_before = Asn1Time::days_from_now(0)
            .map_err(|e| GenesisError::Secret(format!("Failed to create not_before: {}", e)))?;
        let not_after = Asn1Time::days_from_now(self.validity_days as u32)
            .map_err(|e| GenesisError::Secret(format!("Failed to create not_after: {}", e)))?;

        builder.set_not_before(&not_before)
            .map_err(|e| GenesisError::Secret(format!("Failed to set not_before: {}", e)))?;
        builder.set_not_after(&not_after)
            .map_err(|e| GenesisError::Secret(format!("Failed to set not_after: {}", e)))?;

        builder.set_pubkey(key)
            .map_err(|e| GenesisError::Secret(format!("Failed to set pubkey: {}", e)))?;

        let basic_constraints = BasicConstraints::new()
            .critical()
            .ca()
            .build()
            .map_err(|e| GenesisError::Secret(format!("Failed to build basic constraints: {}", e)))?;
        builder.append_extension(basic_constraints)
            .map_err(|e| GenesisError::Secret(format!("Failed to append basic constraints: {}", e)))?;

        let key_usage = KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()
            .map_err(|e| GenesisError::Secret(format!("Failed to build key usage: {}", e)))?;
        builder.append_extension(key_usage)
            .map_err(|e| GenesisError::Secret(format!("Failed to append key usage: {}", e)))?;

        builder.sign(key, MessageDigest::sha256())
            .map_err(|e| GenesisError::Secret(format!("Failed to sign certificate: {}", e)))?;

        Ok(builder.build())
    }

    fn generate_self_signed(&self, key: &PKey<Private>) -> Result<X509> {
        let mut builder = X509Builder::new()
            .map_err(|e| GenesisError::Secret(format!("Failed to create X509 builder: {}", e)))?;

        builder.set_version(2)
            .map_err(|e| GenesisError::Secret(format!("Failed to set version: {}", e)))?;

        let serial = BigNum::from_u32(1)
            .map_err(|e| GenesisError::Secret(format!("Failed to create serial: {}", e)))?;
        builder.set_serial_number(&serial.to_asn1_integer()
            .map_err(|e| GenesisError::Secret(format!("Failed to set serial: {}", e)))?)
            .map_err(|e| GenesisError::Secret(format!("Failed to set serial number: {}", e)))?;

        let name = self.build_name()?;
        builder.set_subject_name(&name)
            .map_err(|e| GenesisError::Secret(format!("Failed to set subject: {}", e)))?;
        builder.set_issuer_name(&name)
            .map_err(|e| GenesisError::Secret(format!("Failed to set issuer: {}", e)))?;

        let not_before = Asn1Time::days_from_now(0)
            .map_err(|e| GenesisError::Secret(format!("Failed to create not_before: {}", e)))?;
        let not_after = Asn1Time::days_from_now(self.validity_days as u32)
            .map_err(|e| GenesisError::Secret(format!("Failed to create not_after: {}", e)))?;

        builder.set_not_before(&not_before)
            .map_err(|e| GenesisError::Secret(format!("Failed to set not_before: {}", e)))?;
        builder.set_not_after(&not_after)
            .map_err(|e| GenesisError::Secret(format!("Failed to set not_after: {}", e)))?;

        builder.set_pubkey(key)
            .map_err(|e| GenesisError::Secret(format!("Failed to set pubkey: {}", e)))?;

        if !self.alternate_names.is_empty() {
            let mut san = SubjectAlternativeName::new();
            for name in &self.alternate_names {
                if name.contains(':') {
                    san.ip(name);
                } else {
                    san.dns(name);
                }
            }
            let extension = san.build(&builder.x509v3_context(None, None))
                .map_err(|e| GenesisError::Secret(format!("Failed to build SAN: {}", e)))?;
            builder.append_extension(extension)
                .map_err(|e| GenesisError::Secret(format!("Failed to append SAN: {}", e)))?;
        }

        builder.sign(key, MessageDigest::sha256())
            .map_err(|e| GenesisError::Secret(format!("Failed to sign certificate: {}", e)))?;

        Ok(builder.build())
    }
}

impl Secret for X509Secret {
    fn secret_type(&self) -> SecretType {
        SecretType::X509
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        if self.common_name.is_empty() {
            return Err(GenesisError::Secret("Common name cannot be empty".to_string()));
        }

        if self.key_size < 2048 {
            return Err(GenesisError::Secret("Key size must be at least 2048 bits".to_string()));
        }

        if self.validity_days <= 0 {
            return Err(GenesisError::Secret("Validity days must be positive".to_string()));
        }

        if self.cert_type == CertType::Signed && self.ca_path.is_none() {
            return Err(GenesisError::Secret("Signed certificates require ca_path".to_string()));
        }

        Ok(())
    }

    fn generate(&self) -> Result<HashMap<String, String>> {
        let key = self.generate_private_key()?;
        let private_pem = key.private_key_to_pem_pkcs8()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode private key: {}", e)))?;

        let cert = match self.cert_type {
            CertType::CA => self.generate_ca(&key)?,
            CertType::SelfSigned => self.generate_self_signed(&key)?,
            CertType::Signed => {
                return Err(GenesisError::Secret(
                    "Signed certificates require CA - not yet implemented in this path".to_string()
                ));
            }
        };

        let cert_pem = cert.to_pem()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode certificate: {}", e)))?;

        let mut result = HashMap::new();
        result.insert("certificate".to_string(), String::from_utf8_lossy(&cert_pem).to_string());
        result.insert("private".to_string(), String::from_utf8_lossy(&private_pem).to_string());

        if self.cert_type == CertType::CA || self.cert_type == CertType::SelfSigned {
            result.insert("ca".to_string(), String::from_utf8_lossy(&cert_pem).to_string());
        }

        Ok(result)
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        if !value.contains_key("certificate") || !value.contains_key("private") {
            return Ok(ValidationResult::Missing);
        }

        let cert_pem = value.get("certificate").unwrap();
        let cert = X509::from_pem(cert_pem.as_bytes())
            .map_err(|e| GenesisError::Secret(format!("Invalid certificate PEM: {}", e)))?;

        let not_after = cert.not_after();
        let now = Asn1Time::days_from_now(0)
            .map_err(|e| GenesisError::Secret(format!("Failed to get current time: {}", e)))?;

        let days_until_expiry = not_after.diff(&now)
            .map_err(|e| GenesisError::Secret(format!("Failed to calculate expiry: {}", e)))?
            .days;

        if days_until_expiry < 0 {
            return Ok(ValidationResult::Error(vec![
                "Certificate has expired".to_string()
            ]));
        }

        if days_until_expiry < 30 {
            return Ok(ValidationResult::Warning(vec![
                format!("Certificate expires in {} days", days_until_expiry)
            ]));
        }

        Ok(ValidationResult::Ok)
    }

    fn required_keys(&self) -> &[&str] {
        &["certificate", "private"]
    }

    fn dependencies(&self) -> Vec<String> {
        if let Some(ref ca_path) = self.ca_path {
            vec![ca_path.clone()]
        } else {
            Vec::new()
        }
    }
}
