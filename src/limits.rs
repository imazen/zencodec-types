//! Resource limits for codec operations.

/// Resource limits for encode/decode operations.
///
/// Used to prevent DoS attacks and resource exhaustion. All fields are optional;
/// `None` means no limit for that resource.
///
/// Codecs enforce what they can — not all codecs support all limit types.
///
/// # Example
///
/// ```
/// use zencodec_types::ResourceLimits;
///
/// let limits = ResourceLimits::none()
///     .with_max_pixels(100_000_000)
///     .with_max_memory(512 * 1024 * 1024);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResourceLimits {
    /// Maximum total pixels (width × height).
    pub max_pixels: Option<u64>,
    /// Maximum memory allocation in bytes.
    pub max_memory_bytes: Option<u64>,
    /// Maximum encoded output size in bytes (encode only).
    pub max_output_bytes: Option<u64>,
    /// Maximum image width in pixels.
    pub max_width: Option<u32>,
    /// Maximum image height in pixels.
    pub max_height: Option<u32>,
    /// Maximum input file size in bytes (decode only).
    pub max_file_size: Option<u64>,
    /// Maximum number of animation frames.
    pub max_frames: Option<u32>,
    /// Maximum total animation duration in milliseconds.
    pub max_duration_ms: Option<u64>,
}

impl ResourceLimits {
    /// No limits (all fields `None`).
    pub fn none() -> Self {
        Self::default()
    }

    /// Set maximum total pixels.
    pub fn with_max_pixels(mut self, max: u64) -> Self {
        self.max_pixels = Some(max);
        self
    }

    /// Set maximum memory allocation in bytes.
    pub fn with_max_memory(mut self, bytes: u64) -> Self {
        self.max_memory_bytes = Some(bytes);
        self
    }

    /// Set maximum encoded output size in bytes.
    pub fn with_max_output(mut self, bytes: u64) -> Self {
        self.max_output_bytes = Some(bytes);
        self
    }

    /// Set maximum image width in pixels.
    pub fn with_max_width(mut self, width: u32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Set maximum image height in pixels.
    pub fn with_max_height(mut self, height: u32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Set maximum input file size in bytes.
    pub fn with_max_file_size(mut self, bytes: u64) -> Self {
        self.max_file_size = Some(bytes);
        self
    }

    /// Set maximum number of animation frames.
    pub fn with_max_frames(mut self, frames: u32) -> Self {
        self.max_frames = Some(frames);
        self
    }

    /// Set maximum total animation duration in milliseconds.
    pub fn with_max_duration(mut self, ms: u64) -> Self {
        self.max_duration_ms = Some(ms);
        self
    }

    /// Whether any limits are set.
    pub fn has_any(&self) -> bool {
        self.max_pixels.is_some()
            || self.max_memory_bytes.is_some()
            || self.max_output_bytes.is_some()
            || self.max_width.is_some()
            || self.max_height.is_some()
            || self.max_file_size.is_some()
            || self.max_frames.is_some()
            || self.max_duration_ms.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_limits() {
        let limits = ResourceLimits::none();
        assert!(!limits.has_any());
    }

    #[test]
    fn builder_sets_limits() {
        let limits = ResourceLimits::none()
            .with_max_pixels(1_000_000)
            .with_max_memory(512 * 1024 * 1024);
        assert!(limits.has_any());
        assert_eq!(limits.max_pixels, Some(1_000_000));
        assert_eq!(limits.max_memory_bytes, Some(512 * 1024 * 1024));
        assert!(limits.max_output_bytes.is_none());
    }

    #[test]
    fn animation_limits() {
        let limits = ResourceLimits::none()
            .with_max_frames(100)
            .with_max_duration(30_000);
        assert!(limits.has_any());
        assert_eq!(limits.max_frames, Some(100));
        assert_eq!(limits.max_duration_ms, Some(30_000));
    }

    #[test]
    fn has_any_includes_animation_fields() {
        let limits = ResourceLimits::none().with_max_frames(10);
        assert!(limits.has_any());

        let limits = ResourceLimits::none().with_max_duration(5000);
        assert!(limits.has_any());
    }
}
