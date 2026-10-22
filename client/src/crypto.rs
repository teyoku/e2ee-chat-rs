use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

pub struct Identity {
    secret: StaticSecret,
    pub public: PublicKey,
}

impl Identity {
    pub fn generate() -> Self {
        let secret = StaticSecret::random();
        let public = PublicKey::from(&secret);

        Self { secret, public }
    }

    pub fn public_hex(&self) -> String {
        hex::encode(self.public.as_bytes())
    }

    pub fn session_key(&self, peer_public: &PublicKey) -> Result<[u8; 32], String> {
        let shared = self.secret.diffie_hellman(peer_public);

        if shared.as_bytes().iter().all(|&b| b == 0) {
            return Err("Invalid peer key".into());
        }

        let hkdf = Hkdf::<Sha256>::new(None, shared.as_bytes());

        let mut key = [0u8; 32];

        hkdf.expand(b"e2ee-chat-rs-v1", &mut key)
            .map_err(|_| "HKDF error")?;

        Ok(key)
    }
}

pub fn encrypt(key: &[u8; 32], plaintext: &str, aad: &[u8]) -> Result<(String, String), String> {
    let cipher = XChaCha20Poly1305::new(key.into());

    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

    let cipher_text = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad,
            },
        )
        .map_err(|_| "Encryption failed")?;

    Ok((hex::encode(nonce), hex::encode(cipher_text)))
}

pub fn decrypt(
    key: &[u8; 32],
    nonce_text: &str,
    ciphertext_hex: &str,
    aad: &[u8],
) -> Result<String, String> {
    let nonce_bytes = hex::decode(nonce_text).map_err(|e| e.to_string())?;

    let ciphertext = hex::decode(ciphertext_hex).map_err(|e| e.to_string())?;

    let nonce_array: [u8; 24] = nonce_bytes.try_into().map_err(|_| "Invalid nonce length")?;

    let cipher = XChaCha20Poly1305::new(key.into());

    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(&nonce_array),
            Payload {
                msg: &ciphertext,
                aad,
            },
        )
        .map_err(|_| "Decryption failed")?;

    String::from_utf8(plaintext).map_err(|e| e.to_string())
}
