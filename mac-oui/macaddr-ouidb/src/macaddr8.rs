use alloc::string::String;
use core::fmt;
use core::ops::Sub;
use core::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::OuiDb;

/// The number of bytes in an EUI-64 (extended) MAC address.
pub const EUI64_ADDR_LEN: usize = 8;

const LOCAL_ADDR_BIT: u8 = 0x02;
const MULTICAST_ADDR_BIT: u8 = 0x01;

/// EUI-64 (64-bit extended) MAC address
///
/// The interface mirrors [`MacAddress`](crate::MacAddress) (the 48-bit type), but
/// the address is 8 bytes wide. OUI lookup only depends on the leading bytes, so
/// the same database and rules apply.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MacAddress8(pub(crate) [u8; 8]);

impl MacAddress8 {
    /// Construct a new `MacAddress8` instance from an 8-byte array.
    pub const fn new(mac: [u8; 8]) -> Self {
        MacAddress8(mac)
    }

    /// Construct a new `MacAddress8` instance from 8 components.
    #[allow(clippy::too_many_arguments)]
    pub const fn new8(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) -> Self {
        MacAddress8([a, b, c, d, e, f, g, h])
    }

    /// Construct a new `MacAddress8` instance from a slice.
    pub const fn from_slice(mac: &[u8]) -> Result<Self, ParseMacError> {
        if mac.len() != 8 {
            return Err(ParseMacError::InvalidLength);
        }
        let bytes = [
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[6], mac[7],
        ];
        Ok(MacAddress8(bytes))
    }

    /// Construct an all-zero `MacAddress8` instance.
    pub const fn zero() -> Self {
        Self([0; EUI64_ADDR_LEN])
    }

    /// Construct a broadcast `MacAddress8` instance.
    pub const fn broadcast() -> Self {
        Self([0xff; EUI64_ADDR_LEN])
    }

    /// Returns true if the address is all-zero.
    pub fn is_zero(&self) -> bool {
        *self == Self::zero()
    }

    /// Returns true if the address is a universally administered address (UAA).
    pub fn is_universal(&self) -> bool {
        !self.is_local()
    }

    /// Returns true if the address is a locally administered address (LAA).
    pub fn is_local(&self) -> bool {
        (self.0[0] & LOCAL_ADDR_BIT) == LOCAL_ADDR_BIT
    }

    /// Returns true if the address is a unicast address.
    pub fn is_unicast(&self) -> bool {
        !self.is_multicast()
    }

    /// Returns true if the address is a multicast address.
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & MULTICAST_ADDR_BIT) == MULTICAST_ADDR_BIT
    }

    /// Returns true if the address is a broadcast address.
    pub fn is_broadcast(&self) -> bool {
        *self == Self::broadcast()
    }

    /// Look up the OUI entry for this address.
    ///
    /// The returned [`OuiEntryOwned`](crate::OuiEntryOwned) owns its data source,
    /// so it remains valid after the global database handle is dropped (or the
    /// database itself is replaced).
    pub fn oui_entry(&self) -> Option<crate::OuiEntryOwned> {
        crate::oui_db().lookup_mac6_owned(*self)
    }

    /// Return true if the address is a virtual NIC.
    pub fn is_virtual_nic(&self) -> bool {
        self.oui_entry()
            .is_some_and(|e| OuiDb::is_virtual_nic(e.name()))
    }

    /// Return the OUI (Organizationally Unique Identifier) name only.
    ///
    /// The name is copied out of the current global database, so the returned
    /// `String` stays valid independently of the database's lifetime.
    pub fn oui(&self) -> Option<String> {
        self.oui_entry().map(|e| String::from(e.name()))
    }

    /// Return the raw octets.
    pub fn octets(&self) -> [u8; 8] {
        self.0
    }

    /// Return the address as a big-endian `u64` (all 8 bytes).
    pub fn to_u64(&self) -> u64 {
        ((self.0[0] as u64) << 56)
            | ((self.0[1] as u64) << 48)
            | ((self.0[2] as u64) << 40)
            | ((self.0[3] as u64) << 32)
            | ((self.0[4] as u64) << 24)
            | ((self.0[5] as u64) << 16)
            | ((self.0[6] as u64) << 8)
            | (self.0[7] as u64)
    }
}

impl From<[u8; 8]> for MacAddress8 {
    fn from(value: [u8; 8]) -> Self {
        Self(value)
    }
}

impl fmt::Debug for MacAddress8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7]
        )
    }
}

impl fmt::Display for MacAddress8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7]
        )
    }
}

impl Sub for &MacAddress8 {
    type Output = i128;

    fn sub(self, rhs: Self) -> Self::Output {
        self.to_u64() as i128 - rhs.to_u64() as i128
    }
}

impl Sub for MacAddress8 {
    type Output = i128;

    fn sub(self, rhs: Self) -> Self::Output {
        self.to_u64() as i128 - rhs.to_u64() as i128
    }
}

impl FromStr for MacAddress8 {
    type Err = ParseMacError;

    /// Parse an 8-byte MAC address, e.g. `52:54:00:12:34:56:78:9A`,
    /// `52-54-00-12-34-56-78-9A`, `001A.2B3C.4D5E.6F70` (Cisco IOS format) or
    /// `525400123456789A` (continuous hex).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try Cisco IOS format first (XXXX.XXXX.XXXX.XXXX)
        let value = Self::from_str_cisco(s);
        if value.is_ok() {
            return value;
        }

        // Try colon-separated format
        let value = Self::from_str_sep(s, ':');
        if value.is_ok() {
            value
        } else {
            // Try hyphen-separated format
            let value = Self::from_str_sep(s, '-');
            if value.is_ok() {
                value
            } else {
                // Try continuous hex format (16 hex digits without separator)
                Self::from_str_continuous(s)
            }
        }
    }
}

impl core::convert::TryFrom<&'_ str> for MacAddress8 {
    type Error = ParseMacError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl core::convert::TryFrom<&'_ [u8]> for MacAddress8 {
    type Error = ParseMacError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::from_slice(value)
    }
}

#[cfg(feature = "std")]
impl core::convert::TryFrom<std::borrow::Cow<'_, str>> for MacAddress8 {
    type Error = ParseMacError;

    fn try_from(value: std::borrow::Cow<'_, str>) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl MacAddress8 {
    /// Parse an 8-byte MAC address with `separator` between octets.
    fn from_str_sep(s: &str, separator: char) -> Result<Self, ParseMacError> {
        let mut parts = [0u8; 8];
        let splits = s.split(separator);
        let mut i = 0;
        for split in splits {
            if i == 8 {
                return Err(ParseMacError::InvalidLength);
            }
            match u8::from_str_radix(split, 16) {
                Ok(b) if !split.is_empty() => parts[i] = b,
                _ => return Err(ParseMacError::InvalidDigit),
            }
            i += 1;
        }

        if i == 8 {
            Ok(Self(parts))
        } else {
            Err(ParseMacError::InvalidLength)
        }
    }

    /// Parse Cisco IOS format: `XXXX.XXXX.XXXX.XXXX` (16 hex digits in 4 groups of 4).
    fn from_str_cisco(s: &str) -> Result<Self, ParseMacError> {
        let len = s.len();
        if len != 19 {
            return Err(ParseMacError::InvalidLength);
        }

        let bytes = s.as_bytes();
        if bytes[4] != b'.' || bytes[9] != b'.' || bytes[14] != b'.' {
            return Err(ParseMacError::InvalidDigit);
        }

        let mut parts = [0u8; 8];
        let idx = [
            (0, 2),
            (2, 4),
            (5, 7),
            (7, 9),
            (10, 12),
            (12, 14),
            (15, 17),
            (17, 19),
        ];
        for (i, (start, end)) in idx.into_iter().enumerate() {
            parts[i] =
                u8::from_str_radix(&s[start..end], 16).map_err(|_| ParseMacError::InvalidDigit)?;
        }
        Ok(Self(parts))
    }

    /// Parse continuous hex format: exactly 16 hex digits without separators.
    fn from_str_continuous(s: &str) -> Result<Self, ParseMacError> {
        let len = s.len();
        if len != 16 {
            return Err(ParseMacError::InvalidLength);
        }

        let mut parts = [0u8; 8];
        for (i, part) in parts.iter_mut().enumerate() {
            *part = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| ParseMacError::InvalidDigit)?;
        }
        Ok(Self(parts))
    }
}

#[cfg(feature = "serde")]
impl Serialize for MacAddress8 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MacAddress8 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MacAddr8Visitor;
        impl<'de> de::Visitor<'de> for MacAddr8Visitor {
            type Value = MacAddress8;

            fn visit_str<E: de::Error>(self, value: &str) -> Result<MacAddress8, E> {
                value.parse().map_err(E::custom)
            }

            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<MacAddress8, E> {
                if v.len() == 8 {
                    Ok(MacAddress8::new([
                        v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
                    ]))
                } else {
                    Err(E::invalid_length(v.len(), &self))
                }
            }

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "either a string representation of a MAC address or 8-element byte array"
                )
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(MacAddr8Visitor)
        } else {
            deserializer.deserialize_bytes(MacAddr8Visitor)
        }
    }
}

use crate::ParseMacError;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn parse_and_display_colon() {
        let mac: MacAddress8 = "52:54:00:12:34:56:78:9A".parse().unwrap();
        assert_eq!(
            mac.octets(),
            [0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x78, 0x9A]
        );
        assert_eq!(format!("{mac}"), "52:54:00:12:34:56:78:9a");
    }

    #[test]
    fn parse_hyphen_and_continuous_and_cisco() {
        let a: MacAddress8 = "52-54-00-12-34-56-78-9A".parse().unwrap();
        let b: MacAddress8 = "525400123456789A".parse().unwrap();
        let c: MacAddress8 = "5254.0012.3456.789A".parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn parse_invalid() {
        assert!("52:54:00:12:34:56:78".parse::<MacAddress8>().is_err());
        assert!("52:54:00:12:34:56:78:9A:BC".parse::<MacAddress8>().is_err());
        assert!("525400123456789".parse::<MacAddress8>().is_err());
        assert!("001G.2B3C.4D5E.6F70".parse::<MacAddress8>().is_err());
    }

    #[test]
    fn flags_and_zero_broadcast() {
        let zero = MacAddress8::zero();
        assert!(zero.is_zero());
        assert!(!zero.is_broadcast());

        let bcast = MacAddress8::broadcast();
        assert!(bcast.is_broadcast());

        let local: MacAddress8 = "02:00:00:00:00:00:00:01".parse().unwrap();
        assert!(local.is_local());
        assert!(!local.is_universal());

        let multi: MacAddress8 = "01:00:00:00:00:00:00:00".parse().unwrap();
        assert!(multi.is_multicast());
        assert!(!multi.is_unicast());
    }

    #[test]
    fn to_u64_roundtrip() {
        let mac: MacAddress8 = "0102030405060708".parse().unwrap();
        assert_eq!(mac.to_u64(), 0x0102030405060708);
    }

    #[test]
    fn lookup_mac6_returns_entry() {
        let mac: MacAddress8 = "00:00:01:00:00:00:00:00".parse().unwrap();
        let entry = mac.oui_entry().expect("XEROX entry");
        assert!(entry.name().starts_with("XEROX"));
    }
}
