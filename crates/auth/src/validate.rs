use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use crate::jwks::JwksClient;

const SELF_SIGNED_KID: &str = "jFNXMnFjGrSoDafnLQBohoCNalWcFcTjnKEbkRzWFBHyYJFikdLMHP";

#[derive(Debug, Clone, Deserialize)]
pub struct TokenClaims {
    pub id: Option<String>,
    pub sub: Option<String>,
}

impl TokenClaims {
    pub fn user_id(&self) -> Option<&str> {
        self.id.as_deref().or(self.sub.as_deref())
    }
}

#[derive(Clone)]
pub struct TokenValidator {
    jwks: JwksClient,
    cookie_secret: String,
    auth0_domain: String,
    auth0_audience: String,
    /// When true, the validator decodes JWT claims WITHOUT verifying the
    /// signature. Intended strictly for local development when aura-storage
    /// is not configured with the real zOS cookie secret.
    /// `main.rs` reads this from `AURA_STORAGE_DEV_TRUST_TOKENS` and logs
    /// a loud warning at startup.
    dev_trust_tokens: bool,
}

impl TokenValidator {
    pub fn new(auth0_domain: String, auth0_audience: String, cookie_secret: String) -> Self {
        Self::with_dev_trust(auth0_domain, auth0_audience, cookie_secret, false)
    }

    pub fn with_dev_trust(
        auth0_domain: String,
        auth0_audience: String,
        cookie_secret: String,
        dev_trust_tokens: bool,
    ) -> Self {
        Self {
            jwks: JwksClient::new(&auth0_domain),
            cookie_secret,
            auth0_domain,
            auth0_audience,
            dev_trust_tokens,
        }
    }

    pub async fn validate(&self, token: &str) -> Result<TokenClaims, String> {
        let header =
            jsonwebtoken::decode_header(token).map_err(|e| format!("Invalid token header: {e}"))?;

        let kid = header.kid.as_deref().unwrap_or("");

        if self.dev_trust_tokens {
            return self.validate_dev_trust(token, header.alg);
        }

        if kid == SELF_SIGNED_KID {
            self.validate_hs256(token)
        } else {
            self.validate_rs256(token, kid).await
        }
    }

    /// Decode claims with no signature verification. Used only when
    /// `dev_trust_tokens` is set via env. Never call this in production.
    fn validate_dev_trust(&self, token: &str, alg: Algorithm) -> Result<TokenClaims, String> {
        // `insecure_disable_signature_validation` still requires a matching
        // algorithm in the Validation struct, so mirror whatever the token
        // header claims. The key bytes are ignored.
        let mut validation = Validation::new(alg);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_aud = false;
        validation.required_spec_claims.clear();

        let dummy = DecodingKey::from_secret(&[]);
        decode::<TokenClaims>(token, &dummy, &validation)
            .map(|data| data.claims)
            .map_err(|e| format!("dev-trust decode failed: {e}"))
    }

    fn validate_hs256(&self, token: &str) -> Result<TokenClaims, String> {
        let key = DecodingKey::from_secret(self.cookie_secret.as_bytes());
        let mut validation = Validation::new(Algorithm::HS256);
        // zOS API sets exp to 1 year from issuance on self-signed tokens
        validation.validate_aud = false;
        validation.required_spec_claims.clear();

        decode::<TokenClaims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|e| format!("HS256 validation failed: {e}"))
    }

    async fn validate_rs256(&self, token: &str, kid: &str) -> Result<TokenClaims, String> {
        let key = self.jwks.get_key(kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.auth0_audience]);
        validation.set_issuer(&[format!("https://{}/", self.auth0_domain)]);

        decode::<TokenClaims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|e| format!("RS256 validation failed: {e}"))
    }
}
