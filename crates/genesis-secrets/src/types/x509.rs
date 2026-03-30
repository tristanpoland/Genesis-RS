//! X.509 certificate secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, IsCa, BasicConstraints, SanType, PKCS_RSA_SHA256, KeyPair};
use rsa::{pkcs8::{EncodePrivateKey, DecodePrivateKey}, RsaPrivateKey};
use rand::rngs::OsRng;
use rsa::traits::PublicKeyParts;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::{OffsetDateTime, Duration as TimeDuration};
use x509_parser::{pem::parse_x509_pem, parse_x509_certificate};

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

        let usage_str = def.remove("usage")
            .and_then(|v| v.as_str().map(String::from));

        let is_server_cert = usage_str.as_ref()
            .map(|s| s.contains("server"))
            .unwrap_or(true);

        let is_client_cert = usage_str.as_ref()
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

    fn generate_key_pair(&self) -> Result<RsaPrivateKey> {
        let mut rng = OsRng;
        RsaPrivateKey::new(&mut rng, self.key_size as usize)
            .map_err(|e| GenesisError::Secret(format!("Failed to generate RSA key: {}", e)))
    }

    fn build_certificate_params(&self) -> Result<CertificateParams> {
        let mut params = CertificateParams::new(vec![self.common_name.clone()]);
        params.alg = &PKCS_RSA_SHA256;

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, self.common_name.clone());
        if let Some(ref o) = self.organization {
            dn.push(DnType::OrganizationName, o.clone());
        }
        if let Some(ref ou) = self.organizational_unit {
            dn.push(DnType::OrganizationalUnitName, ou.clone());
        }
        if let Some(ref c) = self.country {
            dn.push(DnType::CountryName, c.clone());
        }
        if let Some(ref st) = self.state {
            dn.push(DnType::StateOrProvinceName, st.clone());
        }
        if let Some(ref l) = self.locality {
            dn.push(DnType::LocalityName, l.clone());
        }
        params.distinguished_name = dn;

        if !self.alternate_names.is_empty() {
            params.subject_alt_names = self.alternate_names.iter().filter_map(|name| {
                if let Ok(addr) = name.parse() {
                    Some(SanType::IpAddress(addr))
                } else {
                    Some(SanType::DnsName(name.clone()))
                }
            }).collect();
        }

        params.not_before = OffsetDateTime::now_utc();
        params.not_after = OffsetDateTime::now_utc() + TimeDuration::days(self.validity_days);

        params.is_ca = if self.cert_type == CertType::CA {
            IsCa::Ca(BasicConstraints::Unconstrained)
        } else {
            IsCa::SelfSignedOnly
        };

        Ok(params)
    }

    fn make_certificate(&self) -> Result<Certificate> {
        let mut params = self.build_certificate_params()?;

        // rcgen doesn't support selecting RSA key size directly in params yet, so we manually generate.
        let private_key = self.generate_key_pair()?;
        let private_der = private_key.to_pkcs8_der()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode private key as PKCS8 DER: {}", e)))?;

        params.key_pair = Some(KeyPair::from_der(private_der.as_bytes())
            .map_err(|e| GenesisError::Secret(format!("Failed to create key pair: {}", e)))?);

        Certificate::from_params(params)
            .map_err(|e| GenesisError::Secret(format!("Failed to build certificate: {}", e)))
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
        if self.cert_type == CertType::Signed {
            return Err(GenesisError::Secret(
                "Signed certificates require CA - not yet implemented in this path".to_string(),
            ));
        }

        let cert = self.make_certificate()?;
        let cert_pem = cert.serialize_pem()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode certificate: {}", e)))?;
        let private_pem = cert.serialize_private_key_pem();

        let mut result = HashMap::new();
        result.insert("certificate".to_string(), cert_pem.clone());
        result.insert("private".to_string(), private_pem.clone());

        if self.cert_type == CertType::CA || self.cert_type == CertType::SelfSigned {
            result.insert("ca".to_string(), cert_pem);
        }

        Ok(result)
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        if !value.contains_key("certificate") || !value.contains_key("private") {
            return Ok(ValidationResult::Missing);
        }

        let cert_pem = value.get("certificate").unwrap();
        let (_, pem) = parse_x509_pem(cert_pem.as_bytes())
            .map_err(|e| GenesisError::Secret(format!("Invalid certificate PEM: {}", e)))?;

        let (_, cert) = parse_x509_certificate(&pem.contents)
            .map_err(|e| GenesisError::Secret(format!("Invalid certificate DER: {}", e)))?;

        let not_after = cert.tbs_certificate.validity.not_after;
        let now = OffsetDateTime::now_utc();

        let not_after_dt = not_after.to_datetime();

        let days_until_expiry = (not_after_dt - now).whole_days();

        if days_until_expiry < 0 {
            return Ok(ValidationResult::Error(vec!["Certificate has expired".to_string()]));
        }

        if days_until_expiry < 30 {
            return Ok(ValidationResult::Warning(vec![format!("Certificate expires in {} days", days_until_expiry)]));
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
