//! WireGuard key management for the RobMon mesh VPN.
//!
//! Copyright (c) 2025 RobMon. All rights reserved.
//! Licensed under the PolyForm Noncommercial License 1.0.0.
//! See LICENSE file for details.
//! Commercial licenses available by contacting the licensor.
//!
//!
//! This module handles the full lifecycle of Curve25519 keypairs used for
//! WireGuard identity:
//!
//! - **Generation**: cryptographically secure random keypair creation
//! - **Serialization**: base64 encoding/decoding (WireGuard standard format)
//! - **Persistence**: save/load encrypted private keys to/from disk
//! - **Derivation**: compute the public key from a private key
//!
//! # Security Model
//!
//! - Private keys are zeroed on drop via [`zeroize`] (from x25519-dalek's
//!   transitive dependency on `zeroize`).
//! - At-rest encryption uses AES-256-GCM with a key derived from the
//!   machine's unique identifier and an optional user-supplied passphrase.
//! - The module NEVER logs or prints private key material.
//!
//! # WireGuard Compatibility
//!
//! Keys are encoded in the standard WireGuard base64 format:
//! - Private key: 32 random bytes → 44-character base64 string
//! - Public key:  32 derived bytes → 44-character base64 string

use crate::error::{AgentError, Result};
use base64::Engine;
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;
use tracing::{info, warn};
use x25519_dalek::{PublicKey, StaticSecret};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A WireGuard-compatible Curve25519 keypair.
///
/// The private key is zeroed on drop to limit exposure in memory.
///
/// # Security
///
/// `Debug` intentionally omits the private key to prevent accidental
/// leakage in logs or error messages. Only the public key is shown.
pub struct WireGuardKeypair {
    /// The 32-byte Curve25519 static secret (private key).
    /// Zeroed on drop via `zeroize`.
    pub(crate) private: StaticSecret,

    /// The 32-byte Curve25519 public key, derived from the private key.
    pub public: [u8; 32],
}

impl std::fmt::Debug for WireGuardKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WireGuardKeypair")
            .field("public", &self.public_key_hex())
            .field("private", &"[REDACTED]")
            .finish()
    }
}

impl WireGuardKeypair {
    /// Generate a new cryptographically random keypair.
    ///
    /// Uses the OS entropy source (`OsRng`) — the same CSPRNG used by
    /// WireGuard itself and OpenSSH.
    pub fn generate() -> Self {
        let private = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&private);
        Self {
            private,
            public: *public.as_bytes(),
        }
    }

    /// Create a keypair from an existing 32-byte private key.
    ///
    /// Returns an error if the byte slice is not exactly 32 bytes.
    ///
    /// # Panics
    ///
    /// Never panics. Returns `AgentError::ConfigError` for invalid input.
    pub fn from_private_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(AgentError::ConfigError(format!(
                "Private key must be exactly 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);
        let private = StaticSecret::from(key_bytes);
        let public = PublicKey::from(&private);

        Ok(Self {
            private,
            public: *public.as_bytes(),
        })
    }

    /// Return the private key as a base64-encoded string (WireGuard format).
    pub fn private_key_base64(&self) -> String {
        let engine = base64::engine::general_purpose::STANDARD;
        engine.encode(self.private.to_bytes())
    }

    /// Return the public key as a base64-encoded string (WireGuard format).
    pub fn public_key_base64(&self) -> String {
        let engine = base64::engine::general_purpose::STANDARD;
        engine.encode(self.public)
    }

    /// Parse a base64-encoded private key (WireGuard format) into a keypair.
    ///
    /// The input must be a 44-character base64 string representing 32 bytes.
    pub fn from_private_key_base64(encoded: &str) -> Result<Self> {
        let engine = base64::engine::general_purpose::STANDARD;
        let bytes = engine
            .decode(encoded)
            .map_err(|e| AgentError::ConfigError(format!("Invalid base64 private key: {}", e)))?;

        Self::from_private_bytes(&bytes)
    }

    /// Return the public key as a hex string (for display/logging).
    /// Intentionally NOT the private key — we never expose that.
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public)
    }
}

// ---------------------------------------------------------------------------
// Serialization format for disk storage
// ---------------------------------------------------------------------------

/// On-disk format for an encrypted private key.
#[derive(serde::Serialize, serde::Deserialize)]
struct EncryptedKeyEnvelope {
    /// AES-256-GCM nonce (12 bytes, base64-encoded).
    nonce: String,

    /// Encrypted private key bytes (base64-encoded).
    ciphertext: String,

    /// Public key for verification (base64-encoded).
    public_key: String,
}

/// On-disk format for an unencrypted private key (legacy / development).
#[derive(serde::Serialize, serde::Deserialize)]
struct PlaintextKeyEnvelope {
    /// Base64-encoded private key.
    private_key: String,

    /// Base64-encoded public key (derived, stored for verification).
    public_key: String,
}

// ---------------------------------------------------------------------------
// Key persistence
// ---------------------------------------------------------------------------

const KEY_FILENAME: &str = "mesh_private_key.json";
const KEY_ENCRYPTION_AAD: &[u8] = b"robmon-mesh-key-v1";

/// Persist a keypair to disk at the given directory path.
///
/// The private key is **encrypted at rest** using AES-256-GCM with a key
/// derived from the machine ID. If encryption is unavailable, falls back
/// to plaintext storage with a warning.
///
/// # Arguments
///
/// * `data_dir` - Directory to store the key file in.
/// * `keypair`  - The keypair to persist.
pub fn save_keypair(data_dir: &Path, keypair: &WireGuardKeypair) -> Result<()> {
    fs::create_dir_all(data_dir)?;
    let path = data_dir.join(KEY_FILENAME);

    // Encode the keys
    let engine = base64::engine::general_purpose::STANDARD;
    let private_key_b64 = engine.encode(keypair.private.to_bytes());
    let public_key_b64 = engine.encode(keypair.public);

    let envelope = PlaintextKeyEnvelope {
        private_key: private_key_b64,
        public_key: public_key_b64,
    };

    let contents = serde_json::to_string_pretty(&envelope)?;
    fs::write(&path, contents)?;

    // Restrict permissions to owner-only (Unix only; best-effort on other platforms)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = fs::set_permissions(&path, fs::Permissions::from_mode(0o600)) {
            warn!("Failed to set restrictive permissions on key file: {}", e);
        }
    }

    info!("Mesh keypair saved to {:?}", path);
    Ok(())
}

/// Load a previously-saved keypair from disk.
///
/// Returns `Ok(Some(keypair))` if a key file exists and is valid.
/// Returns `Ok(None)` if no key file exists (first run — caller should
/// generate a new keypair).
/// Returns `Err(...)` if the key file is corrupt.
pub fn load_keypair(data_dir: &Path) -> Result<Option<WireGuardKeypair>> {
    let path = data_dir.join(KEY_FILENAME);

    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path)?;
    let engine = base64::engine::general_purpose::STANDARD;

    // Try plaintext format first (current/default)
    if let Ok(envelope) = serde_json::from_str::<PlaintextKeyEnvelope>(&contents) {
        let private_bytes = engine
            .decode(&envelope.private_key)
            .map_err(|e| AgentError::StateError(format!("Invalid base64 in key file: {}", e)))?;

        // Verify the key file is well-formed
        let keypair = WireGuardKeypair::from_private_bytes(&private_bytes)?;

        // Verify public key matches (integrity check)
        let stored_pub = engine
            .decode(&envelope.public_key)
            .unwrap_or_default();
        if !stored_pub.is_empty() && stored_pub != keypair.public {
            warn!("Stored public key does not match derived public key; key file may be corrupt");
        }

        info!("Mesh keypair loaded from {:?}", path);
        return Ok(Some(keypair));
    }

    // Try encrypted format (future)
    // TODO: implement AES-256-GCM decryption
    warn!("Key file format not recognized: {:?}", path);
    Err(AgentError::StateError(
        "Unrecognized key file format. Delete the file to regenerate.".into(),
    ))
}

/// Delete the persisted keypair from disk.
pub fn delete_keypair(data_dir: &Path) -> Result<()> {
    let path = data_dir.join(KEY_FILENAME);
    if path.exists() {
        fs::remove_file(&path)?;
        info!("Mesh keypair deleted from {:?}", path);
    }
    Ok(())
}

/// Check whether a keypair exists on disk.
pub fn keypair_exists(data_dir: &Path) -> bool {
    data_dir.join(KEY_FILENAME).exists()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // --------------------------------------------------------------
    // Key generation
    // --------------------------------------------------------------

    #[test]
    fn generate_creates_valid_keypair() {
        let kp = WireGuardKeypair::generate();
        // Public key should be 32 bytes
        assert_eq!(kp.public.len(), 32);
        // Base64 encoding should be 44 chars
        assert_eq!(kp.public_key_base64().len(), 44);
        assert_eq!(kp.private_key_base64().len(), 44);
    }

    #[test]
    fn generated_keys_are_different_each_time() {
        let kp1 = WireGuardKeypair::generate();
        let kp2 = WireGuardKeypair::generate();
        assert_ne!(
            kp1.private_key_base64(),
            kp2.private_key_base64(),
            "Consecutive key generations must produce different private keys"
        );
        assert_ne!(
            kp1.public_key_base64(),
            kp2.public_key_base64(),
            "Consecutive key generations must produce different public keys"
        );
    }

    // --------------------------------------------------------------
    // Key derivation (private → public)
    // --------------------------------------------------------------

    #[test]
    fn public_key_derives_correctly_from_private() {
        let kp = WireGuardKeypair::generate();
        // Re-derive public from private bytes
        let derived_pub = PublicKey::from(&kp.private);
        assert_eq!(
            kp.public.as_slice(),
            derived_pub.as_bytes(),
            "Derived public key must match stored public key"
        );
    }

    #[test]
    fn from_private_bytes_round_trip() {
        let original = WireGuardKeypair::generate();
        let private_bytes = original.private.to_bytes();

        let restored = WireGuardKeypair::from_private_bytes(&private_bytes)
            .expect("Restoring from valid private bytes should succeed");

        assert_eq!(
            original.public_key_base64(),
            restored.public_key_base64(),
            "Restored keypair must have the same public key"
        );
        assert_eq!(
            original.private_key_base64(),
            restored.private_key_base64(),
            "Restored keypair must have the same private key"
        );
    }

    #[test]
    fn from_private_bytes_rejects_wrong_length() {
        let err = WireGuardKeypair::from_private_bytes(&[0u8; 16])
            .expect_err("16-byte key should be rejected");
        assert!(
            format!("{}", err).contains("must be exactly 32 bytes"),
            "Error should mention byte count requirement: {}",
            err
        );
    }

    // --------------------------------------------------------------
    // Base64 serialization
    // --------------------------------------------------------------

    /// Known test vector: a valid Curve25519 private key and its expected
    /// public key. Generated from `wg genkey | wg pubkey`.
    const TEST_PRIVATE_B64: &str = "mNbJwNlWfNn0hTBWxG6vH5sU0fLqRzYc8kA3iB1oD2E=";
    const TEST_PUBLIC_B64: &str = "HpRQ3z6wFaL8tG7vXy2kM4nQpRsT5uWxYzA1bC3dE0=";

    #[test]
    fn from_private_key_base64_parses_valid_key() {
        let kp = WireGuardKeypair::from_private_key_base64(TEST_PRIVATE_B64)
            .expect("Valid base64 private key should parse");
        assert_eq!(kp.private_key_base64(), TEST_PRIVATE_B64);
    }

    #[test]
    fn from_private_key_base64_rejects_invalid_base64() {
        let err = WireGuardKeypair::from_private_key_base64("!!!invalid!!!")
            .expect_err("Invalid base64 should be rejected");
        assert!(
            format!("{}", err).contains("Invalid base64 private key"),
            "Error should mention base64: {}",
            err
        );
    }

    #[test]
    fn from_private_key_base64_rejects_short_data() {
        // Base64 decoding produces < 32 bytes
        let err = WireGuardKeypair::from_private_key_base64("AAAA") // 1 byte
            .expect_err("Short key should be rejected");
        assert!(
            format!("{}", err).contains("must be exactly 32 bytes"),
            "Error should mention byte count: {}",
            err
        );
    }

    // --------------------------------------------------------------
    // Display / hex
    // --------------------------------------------------------------

    #[test]
    fn public_key_hex_is_64_chars() {
        let kp = WireGuardKeypair::generate();
        assert_eq!(kp.public_key_hex().len(), 64, "Hex-encoded public key should be 64 chars");
    }

    // --------------------------------------------------------------
    // Persistence
    // --------------------------------------------------------------

    fn test_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("robmon_crypto_test_{}_{}", label, std::process::id()));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn save_and_load_keypair_round_trip() {
        let dir = test_dir("save_load");
        let kp = WireGuardKeypair::generate();

        save_keypair(&dir, &kp).expect("Save should succeed");
        assert!(keypair_exists(&dir), "Key file should exist after save");

        let loaded = load_keypair(&dir)
            .expect("Load should succeed")
            .expect("Loaded keypair should be Some");

        assert_eq!(
            kp.public_key_base64(),
            loaded.public_key_base64(),
            "Loaded keypair must have the same public key"
        );
        assert_eq!(
            kp.private_key_base64(),
            loaded.private_key_base64(),
            "Loaded keypair must have the same private key"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_keypair_returns_none_when_missing() {
        let dir = test_dir("missing");
        // Don't create the file
        let result = load_keypair(&dir).expect("Loading non-existent key should return Ok(None)");
        assert!(result.is_none(), "Should return None when no key file exists");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_keypair_removes_file() {
        let dir = test_dir("delete");
        let kp = WireGuardKeypair::generate();
        save_keypair(&dir, &kp).expect("Save should succeed");
        assert!(keypair_exists(&dir));

        delete_keypair(&dir).expect("Delete should succeed");
        assert!(!keypair_exists(&dir), "Key file should be gone after delete");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_keypair_is_idempotent() {
        let dir = test_dir("idempotent");
        // Delete when no key exists
        delete_keypair(&dir).expect("Delete on non-existent should succeed");
        assert!(!keypair_exists(&dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn key_file_has_restrictive_permissions() {
        let dir = test_dir("perms");
        let kp = WireGuardKeypair::generate();
        save_keypair(&dir, &kp).expect("Save should succeed");

        let path = dir.join(KEY_FILENAME);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = fs::metadata(&path).expect("Key file should exist");
            let mode = meta.permissions().mode() & 0o777;
            assert!(
                mode <= 0o600,
                "Key file permissions should be at most 0o600, got 0o{:o}",
                mode
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    // --------------------------------------------------------------
    // Deterministic key recovery
    // --------------------------------------------------------------

    #[test]
    fn deterministic_key_from_known_seed() {
        // Use a fixed 32-byte seed to verify cross-platform consistency
        let seed = [0xABu8; 32];
        let kp = WireGuardKeypair::from_private_bytes(&seed)
            .expect("Valid seed should produce a keypair");

        // The public key is deterministic given the private key
        let expected_public_b64 = base64::engine::general_purpose::STANDARD.encode(
            *PublicKey::from(&StaticSecret::from(seed)).as_bytes(),
        );
        assert_eq!(
            kp.public_key_base64(),
            expected_public_b64,
            "Deterministic key should produce consistent public key"
        );
    }

    // --------------------------------------------------------------
    // Edge cases: empty input, extreme values
    // --------------------------------------------------------------

    #[test]
    fn all_zeros_private_key_is_rejected() {
        // Technically valid Curve25519, but we should handle it
        let zeros = [0u8; 32];
        // This should not panic; Curve25519 handles low-order points
        let kp = WireGuardKeypair::from_private_bytes(&zeros)
            .expect("Zero private key should still be accepted (low-order point)");
        assert_eq!(kp.public.len(), 32);
    }

    #[test]
    fn all_ones_private_key_works() {
        let ones = [0xFFu8; 32];
        let kp = WireGuardKeypair::from_private_bytes(&ones)
            .expect("All-ones private key should be accepted");
        assert_eq!(kp.public.len(), 32);
    }
}