# Release notes for the Secret Toolkit

## Unreleased

## v0.10.0

### Features

- Bumped `cosmwasm-std` version to `v1.1.11` ([#93]).

### Breaking

- Added optional `admin` field to `utils::InitCallback::to_cosmos_msg` ([#93]).

### Bug fixes

- Only padding encrypted attributes in `utils::pad_handle_result` ([#92]).

[#92]: https://github.com/scrtlabs/secret-toolkit/pull/92
[#93]: https://github.com/scrtlabs/secret-toolkit/pull/93

## v0.9.0

### Features

- Replace `cosmwasm-std` with `secret-cosmwasm-std` in prep for crates.io packages ([#87](https://github.com/scrtlabs/secret-toolkit/pull/87)).
- Add `RngCore` & `CryptoRng` trait to `Prng` ([#87](https://github.com/scrtlabs/secret-toolkit/pull/87)).
- Added `from_env` function for `ContractPrng` that consumes `env.block.random` ([#87](https://github.com/scrtlabs/secret-toolkit/pull/87)).

### Breaking

- Renamed `Prng` as `ContractPrng` ([#87](https://github.com/scrtlabs/secret-toolkit/pull/87)).

## v0.8.2

### Bug fixes

- Fixed a remove bug in `Keymap` and `Keyset` ([#86](https://github.com/scrtlabs/secret-toolkit/pull/86)).

## v0.8.1

### Bug fixes

- Fixed a bug in `Keymap` and `Keyset` ([#84](https://github.com/scrtlabs/secret-toolkit/pull/84)).

### Features

- SecureItem - storage access pattern obfuscating Item ([#82](https://github.com/scrtlabs/secret-toolkit/pull/82)).
- Change the internal `rng` field of the `Prng` struct to be public ([#81](https://github.com/scrtlabs/secret-toolkit/pull/81)),

## v0.8.0

This release upgrades all `secret-toolkit` packages to be compatible with Cosmwasm v1.1.
The APIs remains the same, but it is necessary to upgrade the contract's `cosmwasm` dependencies to `v1.1.0`

### Breaking

- Since `cosmwasm v1.1` had some breaking changes to it's dependencies, this version will not work with `cosmwasm v1`. It is necessary to upgrade to `cosmwasm v1.1` in order to use this release and vice verca. However, neither `cosmwasm v1.1` or this version did not have breaking changes to the APIs.

## v0.7.0

- This release changes the internal toolkit package to be part of the workspace - this fixes default-features flags in some of the crates. In addition, crates used by the toolkit have been bumped, and the edition of the toolkit crates has been bumped to 2021.

- Added the `Keyset` storage object (A hashset like storage object).
- Allowed further customisation of Keymap and Keyset with new constructor structs called `KeymapBuilder` and `KeysetBuilder` which allow the user to disable the iterator feature (saving gas) or adjust the internal indexes' page size so that the user may determine how many objects are to be stored/loaded together in the iterator.
- `::new_with_page_size(namespace, page_size)` method was added to `AppendStore` and `DequeStore` so that the user may adjust the internal indexes' page size which determine how many objects are to be stored/loaded together in the iterator.
- Minor performance upgrades to `Keymap`, `AppendStore`, and `DequeStore`.

### Breaking

- Older rust compilers ( < 1.50 ) may not work due to upgraded dependencies

## v0.6.0

This release upgrades all `secret-toolkit` packages to be compatible with Cosmwasm v1.0 (Secret Network v1.4).
The APIs remains the same, but it is necessary to upgrade the contract's `cosmwasm` dependencies to `v1.0.0`.

### Breaking

- This version will not work with `cosmwasm v0.10`. It is necessary to upgrade to `cosmwasm v1` in order to use this release.

## v0.5.0

This release includes some minor fixed to the storage package which required some breaking changes.
We are releasing these breaking changes because we reached the conclusion that the current interfaces
are prone to bugs, or inefficient. Unless you are using these specific interfaces, you should be able to upgrade from 0.4 without issues.

### Breaking

- Removed the implementations of Clone for storage types which are not useful and may cause data corruption if used incorrectly.
- Changed `Keymap::insert` to take the item by reference rather than by value. This should reduce the cost of calling that function by avoiding cloning.

### Features

- Changed the implementation of the `add_prefix` methods in the storage package to use length prefixing, which should help avoid namespace collisions.

## secret-toolkit-storage v0.4.2

- BUGFIX: implementation of `.clone` method fixed
- Added `.add_suffix` and `.clone` methods to `secret-toolkit::storage::Item`
- Minor performance updates to `secret-toolkit::storage::Keymap`

## secret-toolkit-storage v0.4.1

- BUGFIX: `Item::is_empty` was returning the opposite value from what you'd expect.

## v0.4.0

This release mostly includes the work of @srdtrk in #53. Thanks Srdtrk!

It revamps the `secret-toolkit-storage` package to make it more similar to `cw-storage-plus` and much easier
to use. It also removes the `Cashmap` type from the incubator in favor of `KeyMap` in `secret-toolkit-storage`.

This is a summary of the changes and additions in this release:

- Minimum Rust version is bumped to the latest v1.63. This is because we want to use `Mutex::new` in a `const fn`.
- No more distinction between `Readonly*` and `*Mut` types. Instead, methods take references or mutable references to the storage every time.
- Usage of `PrefixedStore` is made mostly unnecessary.
- Storage type's constructors are const functions, which means they can be initialized as global static variables.
- Added `secret-toolkit::storage::Item` which is similar to `Item` from `cw-storage-plus` or `TypedStore` from `cosmwasm_storage` v0.10.
- Added `secret-toolkit::storage::KeyMap` which is similar to `Cashmap`.
- `Cashmap` is completely removed.

A full guide to using the new `storage` types can be found
[in the package's readme file](https://github.com/srdtrk/secret-toolkit/blob/3725530aebe149d14f7f3f1662844340eb27e015/packages/storage/Readme.md).

## secret-toolkit-incubator v0.3.1

- Fixed compilation issue with Rust v1.61 (#46, #48)
- Removed Siphasher dependency (#46, #48)

## secret-toolkit-utils v0.3.1

### Security

- BUGFIX: `secret-toolkit::utils::FeatureToggle::handle_pause` had an inverse authorization check: only non-pausers
  could pause features.

## secret-toolkit-permit v0.3.1

- Removed the `ecc-secp256k1` feature from `secret-toolkit-crypto` dependency of `secret-toolkit-permit`.
  - This tiny change significantly reduces the size of binaries that only use the permit feature.

## v0.3.0

- Added `clear` method to `AppendStore` and `DequeStore` to quickly reset the collections (#34)
- docs.rs documentation now includes all sub-crates.
- BUGFIX: `secret-toolkit::snip721::Metadata` was severely out of date with the SNIP-721 specification, and not useful.
  It is now compatible with deployed SNIP-721 contracts.

- Added `types` module under the `util` package, to standardize often used types.
- Added `secret-toolkit::viewing_key`, which can be imported by enabling the `viewing-key` feature.
- Added `secret-toolkit::permit::PubKey::canonical_address()`.
- Types in `secret-toolkit::permit::Permit` are now generic over the type of permissions they accept.
- Added the `maxheap` type to the incubator.
- Added `secret-toolkit::utils::feature_toggle` which allow managing feature flags in your contract.

### Breaking

- `secret-toolkit::permit::validate()` Now supports validating any type of Cosmos address.
  Interface changes: Now takes a reference to the current token address instead
  of taking it by value and an optional hrp string.
  In addition, it returns a String and not HumanAddr.

- Renamed `secret-toolkit::permit::Permission` to `secret-toolkit::permit::TokenPermission`.
- `secret-toolkit-crypto` now has features `["hash", "rng" and "ecc-secp256k1"]` which are all off by default - enable those you need.
- `secret-toolkit-crypto::secp256k1::PublicKey::parse` now returns `StdResult<Self>`.
- Changes to `secret-toolkit::crypto::secp256k1::PrivateKey::sign`:
  - The `data` argument is now any slice of bytes, and not the hash of a slice of data.
  - the `Api` from `deps.api` is now required as the second argument as we now use the precompiled implementation.
- Changes to `secret-toolkit::crypto::secp256k1::PublicKey::verify`:
  - the `Api` from `deps.api` is now required as the third argument as we now use the precompiled implementation.
- `secret-toolkit-incubator` now has features `["cashmap", "generational-store"]` which are all off by default.

## v0.2.0

This release includes a ton of new features, and a few breaking changes in various interfaces.
This version is also the first released to [crates.io](https://crates.io)!

- Change: when a query fails because of a bad viewing key, this now correctly fails with `StdError::Unauthorized`
- Added support for some missing SNIP-20 functionality, such as `CreateViewingKey`
- Added support for SNIP-21 queries (memos and improved history) which broke some interfaces
- Added support for SNIP-22 messages (batch operations)
- Added support for SNIP-23 messages (improved Send operations) which broke some interfaces
- Added support for SNIP-24 permits
- Added `Base64Of<S: Serde, T>`, `Base64JsonOf<T>`, and `Base64Bincode2Of<T>`,
  which are wrappers that automatically deserializes base64 strings to `T`.
  It can be used in message types' fields instead of `Binary` when the contents of the string
  should have more specific contents.

- Added `storage::DequeStore` - Similar to `AppendStore` but allows pushing and popping on both ends
- Added the `secret-toolkit::incubator` package intended for experimental features. It contains:
  - `CashMap` - A hashmap like storage abstraction
  - `GenerationalIndex` - A generational index storage abstraction
- The various subpackages can now be selected using feature flags. The default flags are `["serialization", "snip20", "snip721", "storage", "utils"]`
  while `["crypto", "permit", "incubator"]` are left disabled by default.

## v0.1.1

- Removed unused dev-dependency that was slowing down test compilation times.

## v0.1.0

This is the first release of `secret-toolkit`. It supports:

- `secret-toolkit::snip20` - Helper types and functions for interaction with
  SNIP-20 contracts.
- `secret-toolkit::snip721` - Helper types and functions for interaction with
  SNIP-721 contracts.
- `secret-toolkit::crypto` - Wrappers for known-to-work crypto primitives from
  ecosystem libraries. We include implementations for Sha256, Secp256k1 keys,
  and ChaChaRng.
- `secret-toolkit::storage` - Types implementing useful storage managements
  techniques: `AppendStore` and `TypedStore`, using `bincode2` by default.
- `secret-toolkit::serialization` - marker types for overriding the storage
  format used by types in `secret-toolkit::storage`. `Json` and `Bincode2`.

- `secret-toolkit::utils` - General utilities for writing contract code.
  - `padding` - tools for padding queries and responses.
  - `calls` - Tools for marking types as messages in queries and callbacks
    to other contracts.
