# Release notes for the Secret Toolkit

## Next Release
TODO: change SecretNetwork dependency after `debug-print` is merged, and before
merging this branch.

* SecretNetwork dependency is now at `v1.0.4-debug-print` allowing usage of
  `debug_print`. Users must set their SecretNetwork dependency to the same tag.

## v0.1.1
* Removed unused dev-dependency that was slowing down test compilation times.

## v0.1.0
This is the first release of `secret-toolkit`. It supports:

* `secret-toolkit::snip20` - Helper types and functions for interaction with
  SNIP-20 contracts.
* `secret-toolkit::snip721` - Helper types and functions for interaction with
  SNIP-721 contracts.
* `secret-toolkit::crypto` - Wrappers for known-to-work crypto primitives from
  ecosystem libraries. We include implementations for Sha256, Secp256k1 keys,
  and ChaChaRng.
* `secret-toolkit::storage` - Types implementing useful storage managements
  techniques: `AppendStore` and `TypedStore`, using `bincode2` by default.
* `secret-toolkit::serialization` - marker types for overriding the storage
  format used by types in `secret-toolkit::storage`. `Json` and `Bincode2`.
* `secret-toolkit::utils` - General utilities for writing contract code.
    * `padding` - tools for padding queries and responses.
    * `calls` - Tools for marking types as messages in queries and callbacks
      to other contracts.
