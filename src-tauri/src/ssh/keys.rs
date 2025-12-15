use ssh_key::rand_core::OsRng;
use ssh_key::{Algorithm, LineEnding, PrivateKey};

pub fn generate_key_pair(
    algorithm: &str,
    passphrase: Option<&str>,
) -> Result<(String, String), String> {
    let mut rng = OsRng;

    let private_key = match algorithm {
        "ed25519" => PrivateKey::random(&mut rng, Algorithm::Ed25519)
            .map_err(|e| format!("Failed to generate Ed25519 key: {}", e))?,
        "rsa" => PrivateKey::random(&mut rng, Algorithm::Rsa { hash: None }) // Default RSA 3072
            .map_err(|e| format!("Failed to generate RSA key: {}", e))?,
        _ => return Err(format!("Unsupported algorithm: {}", algorithm)),
    };

    // Encrypt validation
    let private_key = if let Some(pass) = passphrase {
        if !pass.is_empty() {
            private_key
                .encrypt(&mut rng, pass)
                .map_err(|e| format!("Failed to encrypt key: {}", e))?
        } else {
            private_key
        }
    } else {
        private_key
    };

    let public_key = private_key.public_key();

    let private_pem = private_key
        .to_openssh(LineEnding::LF)
        .map_err(|e| format!("Failed to encode private key: {}", e))?
        .to_string();

    let public_openssh = public_key
        .to_openssh()
        .map_err(|e| format!("Failed to encode public key: {}", e))?;

    Ok((private_pem, public_openssh))
}
