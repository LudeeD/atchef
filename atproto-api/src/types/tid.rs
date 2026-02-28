use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Error;

/// Base32 sortable characters (same as @atproto/common)
const BASE32_CHARS: &[u8; 32] = b"234567abcdefghijklmnopqrstuvwxyz";

/// Timestamp-based ID for ATProto records.
/// Encodes microsecond timestamp + clock sequence in 13 base32-sortable characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tid {
    timestamp_us: u64,
    clock_id: u32,
}

static CLOCK_ID: AtomicU32 = AtomicU32::new(0);
static LAST_TIMESTAMP: AtomicU32 = AtomicU32::new(0);

impl Tid {
    /// Create a new TID for the current timestamp.
    pub fn now() -> Self {
        let timestamp_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_micros() as u64;

        // Get a unique clock_id to avoid collisions within the same microsecond
        let ts_low = (timestamp_us & 0xFFFFFFFF) as u32;
        let last = LAST_TIMESTAMP.swap(ts_low, Ordering::SeqCst);

        let clock_id = if last == ts_low {
            CLOCK_ID.fetch_add(1, Ordering::SeqCst) & 0x3FF // 10 bits
        } else {
            CLOCK_ID.store(0, Ordering::SeqCst);
            0
        };

        Self { timestamp_us, clock_id }
    }

    /// Create a TID from raw components (for testing).
    pub fn from_parts(timestamp_us: u64, clock_id: u32) -> Self {
        Self {
            timestamp_us,
            clock_id: clock_id & 0x3FF,
        }
    }

    /// Get the timestamp in microseconds since Unix epoch.
    pub fn timestamp_us(&self) -> u64 {
        self.timestamp_us
    }

    /// Get the clock ID component.
    pub fn clock_id(&self) -> u32 {
        self.clock_id
    }

    /// Encode to base32 sortable string (13 characters).
    fn encode(&self) -> String {
        // TID is 64 bits: 54-bit timestamp + 10-bit clock_id
        let value = (self.timestamp_us << 10) | (self.clock_id as u64);

        let mut result = [0u8; 13];
        let mut v = value;

        // Encode from right to left (least significant first)
        for i in (0..13).rev() {
            result[i] = BASE32_CHARS[(v & 0x1F) as usize];
            v >>= 5;
        }

        String::from_utf8(result.to_vec()).unwrap()
    }

    /// Decode from base32 sortable string.
    fn decode(s: &str) -> Result<Self, Error> {
        if s.len() != 13 {
            return Err(Error::InvalidTid(format!(
                "TID must be 13 characters, got {}",
                s.len()
            )));
        }

        let mut value: u64 = 0;
        for c in s.chars() {
            let idx = BASE32_CHARS
                .iter()
                .position(|&x| x == c as u8)
                .ok_or_else(|| Error::InvalidTid(format!("invalid character '{}'", c)))?;
            value = (value << 5) | (idx as u64);
        }

        let clock_id = (value & 0x3FF) as u32;
        let timestamp_us = value >> 10;

        Ok(Self { timestamp_us, clock_id })
    }
}

impl fmt::Display for Tid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl FromStr for Tid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tid::decode(s)
    }
}

impl serde::Serialize for Tid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.encode())
    }
}

impl<'de> serde::Deserialize<'de> for Tid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Tid::decode(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tid_roundtrip() {
        let tid = Tid::from_parts(1234567890123456, 42);
        let encoded = tid.to_string();
        let decoded: Tid = encoded.parse().unwrap();
        assert_eq!(tid.timestamp_us(), decoded.timestamp_us());
        assert_eq!(tid.clock_id(), decoded.clock_id());
    }

    #[test]
    fn test_tid_length() {
        let tid = Tid::now();
        assert_eq!(tid.to_string().len(), 13);
    }

    #[test]
    fn test_tid_sortable() {
        let tid1 = Tid::from_parts(1000000, 0);
        let tid2 = Tid::from_parts(2000000, 0);
        assert!(tid1.to_string() < tid2.to_string());
    }

    #[test]
    fn test_invalid_tid() {
        assert!(Tid::from_str("short").is_err());
        assert!(Tid::from_str("0000000000001").is_err()); // contains '0' and '1'
    }
}
