use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{
    encode,
    jwk::{AlgorithmParameters, EllipticCurve, EllipticCurveKeyParameters, Jwk},
    Algorithm, EncodingKey, Header,
};
use p256::ecdsa::SigningKey;
use p256::pkcs8::EncodePrivateKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub struct DpopKeyPair {
    pub private_key_pem: String,
    pub public_jwk: Jwk,
}

pub fn generate_keypair() -> Result<DpopKeyPair> {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_key_pem = signing_key
        .to_pkcs8_pem(Default::default())
        .context("failed to encode private key")?
        .to_string();

    let point = verifying_key.to_encoded_point(false);
    let x = URL_SAFE_NO_PAD.encode(point.x().unwrap());
    let y = URL_SAFE_NO_PAD.encode(point.y().unwrap());

    let public_jwk = Jwk {
        common: Default::default(),
        algorithm: AlgorithmParameters::EllipticCurve(EllipticCurveKeyParameters {
            key_type: jsonwebtoken::jwk::EllipticCurveKeyType::EC,
            curve: EllipticCurve::P256,
            x,
            y,
        }),
    };

    Ok(DpopKeyPair {
        private_key_pem,
        public_jwk,
    })
}

#[derive(Serialize, Deserialize)]
struct DpopClaims {
    jti: String,
    htm: String,
    htu: String,
    iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ath: Option<String>,
}

pub fn create_proof(
    private_key_pem: &str,
    public_jwk: &Jwk,
    http_method: &str,
    http_uri: &str,
    nonce: Option<&str>,
    access_token: Option<&str>,
) -> Result<String> {
    let ath = access_token.map(|token| {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let hash = URL_SAFE_NO_PAD.encode(hasher.finalize());
        tracing::debug!(
            "DPoP ath calculated: token_prefix={} ath={}",
            &token[..token.len().min(20)],
            &hash
        );
        hash
    });

    let claims = DpopClaims {
        jti: Uuid::new_v4().to_string(),
        htm: http_method.to_uppercase(),
        htu: http_uri.to_string(),
        iat: chrono::Utc::now().timestamp(),
        nonce: nonce.map(String::from),
        ath: ath.clone(),
    };

    tracing::debug!(
        "DPoP proof claims: htm={}, htu={}, iat={}, nonce={:?}, ath={:?}",
        claims.htm,
        claims.htu,
        claims.iat,
        claims.nonce,
        claims.ath.as_ref().map(|_| "[present]")
    );

    let mut header = Header::new(Algorithm::ES256);
    header.typ = Some("dpop+jwt".to_string());
    header.jwk = Some(public_jwk.clone());

    let encoding_key =
        EncodingKey::from_ec_pem(private_key_pem.as_bytes()).context("invalid private key")?;

    let proof = encode(&header, &claims, &encoding_key).context("failed to encode DPoP proof")?;
    tracing::debug!("DPoP proof generated: {}...", &proof[..proof.len().min(50)]);
    Ok(proof)
}
