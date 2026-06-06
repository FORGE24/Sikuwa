//! Version and release metadata for Sikuwa 2.0.

/// Semantic version string (keep in sync with `[workspace.package].version`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Release codename.
pub const CODENAME: &str = "Sikuwa 2026/6/6 Ver.A2";

/// Internal engine identifier.
pub const ENGINE: &str = "a2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const CURRENT: Self = Self {
        major: 2,
        minor: 0,
        patch: 0,
    };

    pub fn parse(s: &str) -> Option<Self> {
        let core = s.split('-').next()?;
        let mut parts = core.split('.');
        Some(Self {
            major: parts.next()?.parse().ok()?,
            minor: parts.next()?.parse().ok()?,
            patch: parts.next()?.parse().ok()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Codename;

impl Codename {
    pub const NAME: &'static str = CODENAME;
    pub const ENGINE: &'static str = ENGINE;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse() {
        let v = Version::parse("2.0.0-alpha.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }
}
