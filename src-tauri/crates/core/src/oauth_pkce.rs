use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkceParams {
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

pub fn generate_pkce() -> PkceParams {
    let code_verifier = generate_code_verifier();
    let code_challenge = derive_code_challenge(&code_verifier);

    PkceParams {
        code_verifier,
        code_challenge,
        code_challenge_method: "S256".to_string(),
    }
}

fn generate_code_verifier() -> String {
    let bytes: [u8; 32] = Rng::r#gen(&mut OsRng);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn derive_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = generate_pkce();
        assert_eq!(pkce.code_challenge_method, "S256");
        assert!(!pkce.code_verifier.is_empty());
        assert!(!pkce.code_challenge.is_empty());
        assert_ne!(pkce.code_verifier, pkce.code_challenge);
    }

    #[test]
    fn test_pkce_uniqueness() {
        let pkce1 = generate_pkce();
        let pkce2 = generate_pkce();
        assert_ne!(pkce1.code_verifier, pkce2.code_verifier);
        assert_ne!(pkce1.code_challenge, pkce2.code_challenge);
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let pkce = generate_pkce();
        let challenge = derive_code_challenge(&pkce.code_verifier);
        assert_eq!(challenge, pkce.code_challenge);
    }

    #[test]
    fn test_verifier_length() {
        let pkce = generate_pkce();
        assert!(pkce.code_verifier.len() >= 43);
        assert!(pkce.code_verifier.len() <= 128);
    }
}
