# Release notes for the Secret Toolkit

## Next Release
* Added `clear` method to `AppendStore` and `DequeStore` to quickly reset the collections (#34)
* docs.rs documentation now includes all sub-crates
* BUGFIX: `secret-toolkit::snip721::Metadata` was severely out of date with the SNIP-721 specification, and not useful.
  It is now compatible with deployed SNIP-721 contracts.
* Added `types` module under the `util` package, to standardize often used types.
* Added `secret-toolkit::viewing_key`, which can be imported by enabling the `viewing-key` feature.
* Types in `secret-toolkit::permit::Permit` are now generic over the type of permissions they accept.

### Breaking
* Renamed `secret-toolkit::permit::Permission` to `secret-toolkit::permit::StandardPermission`.

## v0.2.0
This release includes a ton of new features, and a few breaking changes in various interfaces.
This version is also the first released to [crates.io](https://crates.io)!

* Change: when a query fails because of a bad viewing key, this now correctly fails with `StdError::Unauthorized`
* Added support for some missing SNIP-20 functionality, such as `CreateViewingKey`
* Added support for SNIP-21 queries (memos and improved history) which broke some interfaces
* Added support for SNIP-22 messages (batch operations)
* Added support for SNIP-23 messages (improved Send operations) which broke some interfaces
* Added support for SNIP-24 permits
* Added `Base64Of<S: Serde, T>`, `Base64JsonOf<T>`, and `Base64Bincode2Of<T>`, 
    which are wrappers that automatically deserializes base64 strings to `T`.
    It can be used in message types' fields instead of `Binary` when the contents of the string
    should have more specific contents.
* Added `storage::DequeStore` - Similar to `AppendStore` but allows pushing and popping on both ends
* Added the `secret-toolkit::incubator` package intended for experimental features. It contains:
  * `CashMap` - A hashmap like storage abstraction
  * `GenerationalIndex` - A generational index storage abstraction
* The various subpackages can now be selected using feature flags. The default flags are `["serialization", "snip20", "snip721", "storage", "utils"]`
    while `["crypto", "permit", "incubator"]` are left disabled by default.

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
