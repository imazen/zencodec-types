# Why `EncodeJob<'a>` and `DecodeJob<'a>` need `where Self: 'a`

## The problem

Default methods on these traits return `Box<dyn Trait + 'a>` — a type-erased
object that promises to be valid for lifetime `'a`. These methods capture `self`
(or values derived from it) inside the box:

```rust
pub trait EncodeJob<'a>: Sized {
    fn dyn_encoder(self) -> Result<Box<dyn DynEncoder + 'a>, BoxedError> {
        let enc = self.encoder()?;
        Ok(Box::new(move |pixels| {
            //         ^^^^ `enc` came from `self`, now lives inside Box<dyn ... + 'a>
            enc.encode(pixels).map_err(|e| Box::new(e) as BoxedError)
        }))
    }
}
```

The `+ 'a` on the box guarantees the closure is valid for `'a`. But `enc` came
from `self`, and `Self` is generic — Rust doesn't know what's inside it.

## What goes wrong without the bound

```rust
struct BadJob<'x> {
    data: &'x [u8],
}

impl<'a> EncodeJob<'a> for BadJob<'_> { ... }
```

If `'x` is shorter than `'a`, calling `dyn_encoder()` would produce a
`Box<dyn DynEncoder + 'a>` that internally holds `&'x [u8]` — a dangling
reference. The box outlives the data it captured.

## The fix

```rust
fn dyn_encoder(self) -> Result<Box<dyn DynEncoder + 'a>, BoxedError>
where
    Self: 'a,  // all references inside Self must outlive 'a
```

`Self: 'a` means "every reference inside the concrete type must live at least
as long as `'a`." Now `BadJob<'x>` can only satisfy `EncodeJob<'a>` when
`'x: 'a` — the compiler rejects the dangling case at the call site.

## Where to apply it

Every default method that captures `self` into a `Box<dyn ... + 'a>` needs this
bound. The existing `dyn_encoder` and `dyn_frame_encoder` already have it. Any
new default methods following the same pattern need it too.

Alternatively, put the bound on the trait itself:

```rust
pub trait EncodeJob<'a>: Sized where Self: 'a { ... }
// or equivalently:
pub trait EncodeJob<'a>: Sized + 'a { ... }
```

This is simpler — one bound covers all current and future default methods. The
tradeoff is that it's slightly more restrictive (applies even to methods that
don't need it), but in practice every `EncodeJob` implementor already satisfies
`Self: 'a` because the job borrows config with lifetime `'a`.

Putting `Self: 'a` on the trait is the recommended approach. It matches the
actual invariant: a job borrowed from a config for `'a` shouldn't contain
references shorter than `'a`.
