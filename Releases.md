# Release notes for the Secret Toolkit

## Next Release

## secret-toolkit-utils v0.3.1

### Security
* BUGFIX: `secret-toolkit::utils::FeatureToggle::handle_pause` had an inverse authorization check: only non-pausers
  could pause features.

## secret-toolkit-permit v0.3.1
* Removed the `ecc-secp256k1` feature from `secret-toolkit-crypto` dependency of `secret-toolkit-permit`.
    * This tiny change significantly reduces the size of binaries that only use the permit feature.

## v0.3.0
* Added `clear` method to `AppendStore` and `DequeStore` to quickly reset the collections (#34)
* docs.rs documentation now includes all sub-crates.
* BUGFIX: `secret-toolkit::snip721::Metadata` was severely out of date with the SNIP-721 specification, and not useful.
  It is now compatible with deployed SNIP-721 contracts.
* Added `types` module under the `util` package, to standardize often used types.
* Added `secret-toolkit::viewing_key`, which can be imported by enabling the `viewing-key` feature.
* Added `secret-toolkit::permit::PubKey::canonical_address()`.
* Types in `secret-toolkit::permit::Permit` are now generic over the type of permissions they accept.
* Added the `maxheap` type to the incubator.
* Added `secret-toolkit::utils::feature_toggle` which allow managing feature flags in your contract.

### Breaking
* `secret-toolkit::permit::validate()` Now supports validating any type of Cosmos address. 
Interface changes: Now takes a reference to the current token address instead 
of taking it by value and an optional hrp string.
In addition, it returns a String and not HumanAddr.
* Renamed `secret-toolkit::permit::Permission` to `secret-toolkit::permit::TokenPermission`.
* `secret-toolkit-crypto` now has features `["hash", "rng" and "ecc-secp256k1"]` which are all off by default - enable those you need.
* `secret-toolkit-crypto::secp256k1::PublicKey::parse` now returns `StdResult<Self>`.
* Changes to `secret-toolkit::crypto::secp256k1::PrivateKey::sign`:
  * The `data` argument is now any slice of bytes, and not the hash of a slice of data.
  * the `Api` from `deps.api` is now required as the second argument as we now use the precompiled implementation.
* Changes to `secret-toolkit::crypto::secp256k1::PublicKey::verify`:
  * the `Api` from `deps.api` is now required as the third argument as we now use the precompiled implementation.
* `secret-toolkit-incubator` now has features `["cashmap", "generational-store"]` which are all off by default.

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
