//! Error chain helpers for codec error inspection.
//!
//! Codec errors are typically nested: `BoxedError` → `At<MyCodecError>` →
//! `LimitExceeded`. The [`find_cause`] function walks the
//! [`source()`](core::error::Error::source) chain to find a specific cause
//! type without knowing the concrete codec error.
//!
//! Works with `thiserror` `#[from]` variants, `whereat::At<E>` wrappers,
//! and any error type that properly implements `source()`.

/// Walk an error's [`source()`](core::error::Error::source) chain to find
/// a cause of type `T`.
///
/// Starts with the error itself, then follows `source()` links. Returns
/// the first match. This is the primary way to inspect errors from
/// dyn-dispatched codec operations without knowing the concrete error type.
///
/// # Works through wrappers
///
/// - **`thiserror`**: `#[from]` variants expose the inner error via `source()`
/// - **`whereat::At<E>`**: delegates `source()` to the inner error
/// - **`BoxedError`**: downcasts the concrete type first, then walks the chain
///
/// # Example
///
/// ```rust,ignore
/// use zc::{find_cause, LimitExceeded, UnsupportedOperation};
///
/// let result = dyn_decoder.decode();
/// if let Err(ref e) = result {
///     if let Some(limit) = find_cause::<LimitExceeded>(&**e) {
///         eprintln!("limit exceeded: {limit}");
///     } else if let Some(op) = find_cause::<UnsupportedOperation>(&**e) {
///         eprintln!("not supported: {op}");
///     }
/// }
/// ```
pub fn find_cause<'a, T: core::error::Error + 'static>(
    mut err: &'a (dyn core::error::Error + 'static),
) -> Option<&'a T> {
    loop {
        if let Some(t) = err.downcast_ref::<T>() {
            return Some(t);
        }
        err = err.source()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use alloc::string::String;
    use core::fmt;

    use crate::{LimitExceeded, UnsupportedOperation};

    // A simple codec error with source() chain via manual impl
    #[derive(Debug)]
    enum TestCodecError {
        Limit(LimitExceeded),
        Unsupported(UnsupportedOperation),
        Other(String),
    }

    impl fmt::Display for TestCodecError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Limit(e) => write!(f, "limit: {e}"),
                Self::Unsupported(e) => write!(f, "unsupported: {e}"),
                Self::Other(s) => write!(f, "other: {s}"),
            }
        }
    }

    impl core::error::Error for TestCodecError {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            match self {
                Self::Limit(e) => Some(e),
                Self::Unsupported(e) => Some(e),
                Self::Other(_) => None,
            }
        }
    }

    #[test]
    fn find_limit_exceeded_direct() {
        let err = LimitExceeded::Width {
            actual: 5000,
            max: 4096,
        };
        let found = find_cause::<LimitExceeded>(&err);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &err);
    }

    #[test]
    fn find_limit_exceeded_through_codec_error() {
        let inner = LimitExceeded::Pixels {
            actual: 100_000_000,
            max: 50_000_000,
        };
        let err = TestCodecError::Limit(inner.clone());
        let found = find_cause::<LimitExceeded>(&err);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &inner);
    }

    #[test]
    fn find_unsupported_through_codec_error() {
        let err = TestCodecError::Unsupported(UnsupportedOperation::AnimationEncode);
        let found = find_cause::<UnsupportedOperation>(&err);
        assert_eq!(found, Some(&UnsupportedOperation::AnimationEncode));
    }

    #[test]
    fn find_cause_returns_none_when_absent() {
        let err = TestCodecError::Other("something else".into());
        assert!(find_cause::<LimitExceeded>(&err).is_none());
        assert!(find_cause::<UnsupportedOperation>(&err).is_none());
    }

    #[test]
    fn find_through_boxed_error() {
        let inner = LimitExceeded::Memory {
            actual: 1_000_000_000,
            max: 512_000_000,
        };
        let err = TestCodecError::Limit(inner.clone());
        let boxed: Box<dyn core::error::Error + Send + Sync> = Box::new(err);

        // BoxedError.as_ref() → &(dyn Error + Send + Sync + 'static)
        // coerces to &(dyn Error + 'static) for find_cause
        let found = find_cause::<LimitExceeded>(&*boxed);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &inner);
    }
}
