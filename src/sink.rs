//! Zero-copy row sink for streaming decode.
//!
//! [`DecodeRowSink`] lets the caller own the output buffer while the codec
//! writes decoded pixels directly into it — no intermediate allocation.
//!
//! # Usage
//!
//! ```text
//! let job = config.job();
//! let info = job.output_info(data)?;
//! let desc = info.descriptor();
//!
//! // Set up sink with known dimensions and format
//! let mut sink = MySink::new(info.width(), info.height(), desc);
//! job.decoder().decode_rows(data, &mut sink)?;
//! ```
//!
//! # Contract
//!
//! - The codec calls [`demand()`](DecodeRowSink::demand) once per strip,
//!   in top-to-bottom order (`y` increases monotonically).
//! - The returned buffer must be at least `min_bytes` bytes.
//! - The codec writes pixels tightly packed: `width × bpp` bytes per row,
//!   `height` rows, no padding between rows.
//! - When `demand()` is called again, the previous buffer has been fully
//!   written. When `decode_rows()` returns, the last buffer has been written.
//! - The pixel format matches what
//!   [`output_info()`](crate::DecodeJob::output_info) returned.

/// Receives decoded rows during streaming decode.
///
/// The codec calls [`demand`](DecodeRowSink::demand) for each strip of rows,
/// writes decoded pixels directly into the returned buffer, then calls
/// `demand` again for the next strip. After [`Decoder::decode_rows()`](crate::Decoder::decode_rows)
/// returns, the last demanded buffer has been fully written.
///
/// This avoids the intermediate copy that a read-only callback would force —
/// the codec writes directly into memory the caller controls.
///
/// # Object safety
///
/// This trait is object-safe. Use `&mut dyn DecodeRowSink` in generic code.
///
/// # Example implementation
///
/// ```
/// use zencodec_types::DecodeRowSink;
///
/// struct CollectSink {
///     buf: Vec<u8>,
///     strips: Vec<(u32, u32)>, // (y, height) of completed strips
/// }
///
/// impl DecodeRowSink for CollectSink {
///     fn demand(&mut self, y: u32, height: u32, min_bytes: usize) -> &mut [u8] {
///         // Previous strip is complete — record it
///         if y > 0 {
///             if let Some(&(prev_y, prev_h)) = self.strips.last() {
///                 let _ = (prev_y, prev_h); // process previous strip here
///             }
///         }
///         self.buf.resize(min_bytes, 0);
///         &mut self.buf
///     }
/// }
/// ```
pub trait DecodeRowSink {
    /// Provide a mutable buffer for decoded rows `y .. y + height`.
    ///
    /// The codec will write decoded pixels directly into this buffer using
    /// tight packing (stride = width × bytes_per_pixel, no row padding).
    ///
    /// `min_bytes` is the minimum buffer size needed (`width × height × bpp`).
    /// The returned slice must be at least `min_bytes` bytes long.
    ///
    /// When this method is called, any buffer returned by a previous call
    /// has been fully written with decoded pixel data.
    fn demand(&mut self, y: u32, height: u32, min_bytes: usize) -> &mut [u8];
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// Verify the basic demand/fill/demand lifecycle works.
    #[test]
    fn demand_lifecycle() {
        struct TestSink {
            buf: Vec<u8>,
            completed: Vec<(u32, u32)>,
        }

        impl DecodeRowSink for TestSink {
            fn demand(&mut self, y: u32, height: u32, min_bytes: usize) -> &mut [u8] {
                // Record the previous strip as completed (if any)
                if y > 0 {
                    // Previous strip exists — we know it's been written
                    self.completed.push((y - height, height));
                }
                self.buf.resize(min_bytes, 0);
                &mut self.buf
            }
        }

        let mut sink = TestSink {
            buf: Vec::new(),
            completed: Vec::new(),
        };

        // Simulate a codec writing 3 strips of 8 rows each, 10 bytes/row
        let row_bytes = 10;
        for strip in 0..3u32 {
            let y = strip * 8;
            let h = 8;
            let buf = sink.demand(y, h, row_bytes * h as usize);
            assert!(buf.len() >= row_bytes * h as usize);
            // Simulate codec writing
            for b in buf.iter_mut() {
                *b = (strip + 1) as u8;
            }
        }
        // After last strip, record it manually (decode_rows would have returned)
        sink.completed.push((16, 8));

        assert_eq!(sink.completed.len(), 3);
        assert_eq!(sink.completed[0], (0, 8));
        assert_eq!(sink.completed[1], (8, 8));
        assert_eq!(sink.completed[2], (16, 8));
    }

    /// Verify object safety — can use as `&mut dyn DecodeRowSink`.
    #[test]
    fn object_safe() {
        struct SimpleSink {
            buf: Vec<u8>,
        }
        impl DecodeRowSink for SimpleSink {
            fn demand(&mut self, _y: u32, _height: u32, min_bytes: usize) -> &mut [u8] {
                self.buf.resize(min_bytes, 0);
                &mut self.buf
            }
        }

        fn use_sink(sink: &mut dyn DecodeRowSink) {
            let buf = sink.demand(0, 8, 80);
            assert!(buf.len() >= 80);
            buf[0] = 42;
        }

        let mut sink = SimpleSink { buf: Vec::new() };
        use_sink(&mut sink);
        assert_eq!(sink.buf[0], 42);
    }

    /// Verify the lending pattern — each demand() call's borrow is independent.
    #[test]
    fn lending_borrow_pattern() {
        struct ReuseSink {
            buf: vec::Vec<u8>,
            call_count: u32,
        }
        impl DecodeRowSink for ReuseSink {
            fn demand(&mut self, _y: u32, _height: u32, min_bytes: usize) -> &mut [u8] {
                self.call_count += 1;
                self.buf.resize(min_bytes, 0);
                &mut self.buf
            }
        }

        let mut sink = ReuseSink {
            buf: Vec::new(),
            call_count: 0,
        };

        // Multiple sequential borrows — each one ends before the next starts
        {
            let buf = sink.demand(0, 4, 40);
            buf.fill(1);
        } // borrow ends

        {
            let buf = sink.demand(4, 4, 40);
            buf.fill(2);
        } // borrow ends

        {
            let buf = sink.demand(8, 4, 40);
            buf.fill(3);
        } // borrow ends

        assert_eq!(sink.call_count, 3);
        // Last write was 3
        assert_eq!(sink.buf[0], 3);
    }
}
