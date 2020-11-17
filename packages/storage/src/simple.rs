use cosmwasm_std::{ReadonlyStorage, Storage};
use std::convert::TryInto;

pub fn get_i8<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<i8> {
    let bytes = storage.get(key.as_bytes());
    bytes
        .map(|bytes| bytes.as_slice().try_into().ok().map(i8::from_be_bytes))
        .flatten()
}

pub fn set_i8<S: Storage>(storage: &mut S, key: &str, value: i8) {
    let bytes = value.to_be_bytes();
    storage.set(key.as_bytes(), &bytes)
}

pub fn get_i16<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<i16> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_i16<S: Storage>(mut storage: &mut S, key: &str, value: i16) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_i32<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<i32> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_i32<S: Storage>(mut storage: &mut S, key: &str, value: i32) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_i64<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<i64> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_i64<S: Storage>(mut storage: &mut S, key: &str, value: i64) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_i128<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<i128> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_i128<S: Storage>(mut storage: &mut S, key: &str, value: i128) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_u8<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<u8> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_u8<S: Storage>(mut storage: &mut S, key: &str, value: u8) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_u16<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<u16> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_u16<S: Storage>(mut storage: &mut S, key: &str, value: u16) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_u32<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<u32> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_u32<S: Storage>(mut storage: &mut S, key: &str, value: u32) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_u64<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<u64> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_u64<S: Storage>(mut storage: &mut S, key: &str, value: u64) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_u128<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<u128> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_u128<S: Storage>(mut storage: &mut S, key: &str, value: u128) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_f32<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<f32> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_f32<S: Storage>(mut storage: &mut S, key: &str, value: f32) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_f64<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<f64> {
    let bytes = storage.get(key.as_bytes());
    todo!()
}

pub fn set_f64<S: Storage>(mut storage: &mut S, key: &str, value: f64) {
    let bytes = todo!();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_string<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<String> {
    let bytes = storage.get(key.as_bytes());
    bytes.map(|bytes| String::from_utf8(bytes).ok()).flatten()
}

pub fn set_string<S: Storage>(mut storage: &mut S, key: &str, value: &str) {
    let bytes = value.as_bytes();
    storage.set(key.as_bytes(), bytes)
}

pub fn get_bytes<S: ReadonlyStorage>(storage: &S, key: &str) -> Option<Vec<u8>> {
    storage.get(key.as_bytes())
}

pub fn set_bytes<S: Storage>(mut storage: &mut S, key: &str, value: &[u8]) {
    storage.set(key.as_bytes(), value)
}
