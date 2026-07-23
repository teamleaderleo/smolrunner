use std::fmt;

use rustix::rand::{GetRandomFlags, getrandom};
use serde::Serialize;

use crate::state::InstallationId;

const INSTALLATION_RANDOM_BYTES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallationIdGenerationError {
    public_message: String,
}

impl InstallationIdGenerationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            public_message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.public_message
    }
}

impl fmt::Display for InstallationIdGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.public_message)
    }
}

impl std::error::Error for InstallationIdGenerationError {}

/// Generate one opaque installation ID from operating-system randomness.
///
/// The resulting identifier contains 48 lowercase hexadecimal characters. It is an ownership
/// identity, not a credential, and is safe to persist in public state records and journals.
///
/// # Errors
///
/// Returns a bounded error when the operating system cannot provide the complete random input or
/// when the generated value unexpectedly violates the accepted installation-ID format.
pub fn generate_installation_id() -> Result<InstallationId, InstallationIdGenerationError> {
    let mut random = [0_u8; INSTALLATION_RANDOM_BYTES];
    let filled = getrandom(&mut random, GetRandomFlags::empty()).map_err(|_| {
        InstallationIdGenerationError::new(
            "could not obtain operating-system randomness for an installation ID",
        )
    })?;
    if filled != random.len() {
        return Err(InstallationIdGenerationError::new(
            "operating-system randomness returned an incomplete installation ID",
        ));
    }
    encode_installation_id(&random)
}

fn encode_installation_id(
    random: &[u8; INSTALLATION_RANDOM_BYTES],
) -> Result<InstallationId, InstallationIdGenerationError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(random.len() * 2);
    for byte in random {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    InstallationId::parse(&encoded).map_err(|_| {
        InstallationIdGenerationError::new(
            "generated installation ID violated the accepted identifier format",
        )
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::state::InstallationId;

    use super::{INSTALLATION_RANDOM_BYTES, encode_installation_id, generate_installation_id};

    #[test]
    fn deterministic_encoding_is_lowercase_fixed_length_hex() {
        let random = [0xab_u8; INSTALLATION_RANDOM_BYTES];
        let installation_id = encode_installation_id(&random).expect("encode installation ID");
        assert_eq!(
            installation_id.as_str(),
            "ab".repeat(INSTALLATION_RANDOM_BYTES)
        );
        assert_eq!(
            installation_id.as_str().len(),
            INSTALLATION_RANDOM_BYTES * 2
        );
    }

    #[test]
    fn operating_system_generation_emits_distinct_valid_ids() {
        let mut generated = BTreeSet::new();
        for _ in 0..16 {
            let installation_id = generate_installation_id().expect("generate installation ID");
            assert_eq!(
                installation_id.as_str().len(),
                INSTALLATION_RANDOM_BYTES * 2
            );
            assert!(
                installation_id
                    .as_str()
                    .bytes()
                    .all(|byte| { byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) })
            );
            assert_eq!(
                InstallationId::parse(installation_id.as_str()).expect("round-trip generated ID"),
                installation_id
            );
            assert!(generated.insert(installation_id.as_str().to_owned()));
        }
    }
}
