//! Resource limits applied while decoding untrusted font containers.

use std::io::{Error, ErrorKind};

/// Upper bounds used by the safe font-loading entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeLimits {
    /// Maximum input buffer accepted by a loader.
    pub max_input_bytes: usize,
    /// Maximum size of one decoded OpenType table.
    pub max_table_bytes: usize,
    /// Maximum total size of data produced by container decompression.
    pub max_total_decompressed_bytes: usize,
    /// Maximum decompressed WOFF metadata size.
    pub max_metadata_bytes: usize,
    /// Maximum decompressed OpenType SVG document size.
    pub max_svg_bytes: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 512 * 1024 * 1024,
            max_table_bytes: 128 * 1024 * 1024,
            max_total_decompressed_bytes: 512 * 1024 * 1024,
            max_metadata_bytes: 16 * 1024 * 1024,
            max_svg_bytes: 32 * 1024 * 1024,
        }
    }
}

impl DecodeLimits {
    pub(crate) fn validate_input(&self, length: usize) -> Result<(), Error> {
        if length > self.max_input_bytes {
            return Err(resource_limit("font input", length, self.max_input_bytes));
        }
        Ok(())
    }
}

pub(crate) fn resource_limit(kind: &str, actual: usize, limit: usize) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("{kind} exceeds decode limit: {actual} > {limit}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_reject_oversized_input() {
        let limits = DecodeLimits {
            max_input_bytes: 3,
            ..DecodeLimits::default()
        };
        let error = limits.validate_input(4).expect_err("input must be limited");
        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }
}
