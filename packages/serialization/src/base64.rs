use std::fmt;
use std::marker::PhantomData;

use serde::{de, ser};

use cosmwasm_std::Binary;

use crate::Serde;

/// Alias of `cosmwasm_std::Binary` for better naming
pub type Base64 = Binary;

/// A wrapper that automatically deserializes base64 strings to `T` using
/// one of the `Serde` types.
/// Use it as a field of your Handle messages (input and output), for
/// example in the `msg` field of the `Receive` interface, to remove the
/// boilerplate of serializing or deserializing the `Binary` to the relevant
/// type `T`.
pub struct Base64Of<S: Serde, T> {
    // This is pub so that users can easily unwrap this if needed,
    // or just swap the entire instance.
    pub inner: T,
    ser: PhantomData<S>,
}

#[cfg(feature = "json")]
pub type Base64JsonOf<T> = Base64Of<crate::Json, T>;

#[cfg(feature = "bincode2")]
pub type Base64Bincode2Of<T> = Base64Of<crate::Bincode2, T>;

impl<S: Serde, T> From<T> for Base64Of<S, T> {
    fn from(other: T) -> Self {
        Self {
            inner: other,
            ser: PhantomData,
        }
    }
}

impl<S: Serde, T> std::ops::Deref for Base64Of<S, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S: Serde, T> std::ops::DerefMut for Base64Of<S, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<Ser: Serde, T: ser::Serialize> ser::Serialize for Base64Of<Ser, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let string = match Ser::serialize(&self.inner) {
            Ok(b) => Binary(b).to_base64(),
            Err(err) => return Err(<S::Error as ser::Error>::custom(err)),
        };
        serializer.serialize_str(&string)
    }
}

impl<'de, S: Serde, T: for<'des> de::Deserialize<'des>> de::Deserialize<'de> for Base64Of<S, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Base64TVisitor::<S, T>::new())
    }
}

struct Base64TVisitor<S: Serde, T> {
    inner: PhantomData<T>,
    ser: PhantomData<S>,
}

impl<S: Serde, T> Base64TVisitor<S, T> {
    fn new() -> Self {
        Self {
            inner: PhantomData,
            ser: PhantomData,
        }
    }
}

impl<'de, S: Serde, T: for<'des> de::Deserialize<'des>> de::Visitor<'de> for Base64TVisitor<S, T> {
    type Value = Base64Of<S, T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("valid base64 encoded string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let binary = Base64::from_base64(v).map_err(|_| {
            //
            E::custom(format!("invalid base64: {}", v))
        })?;
        match S::deserialize::<T>(binary.as_slice()) {
            Ok(val) => Ok(Base64Of::from(val)),
            Err(err) => Err(E::custom(err)),
        }
    }
}

/// These traits are conditionally implemented for Base64Of<S, T>
/// if T implements the trait being implemented.
mod passthrough_impls {
    use std::cmp::Ordering;
    use std::fmt::{Debug, Display, Formatter};
    use std::hash::{Hash, Hasher};
    use std::marker::PhantomData;

    use schemars::gen::SchemaGenerator;
    use schemars::schema::Schema;
    use schemars::JsonSchema;

    use cosmwasm_std::Binary;

    use crate::Serde;

    use super::Base64Of;

    // Clone
    impl<S: Serde, T: Clone> Clone for Base64Of<S, T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                ser: self.ser,
            }
        }
    }

    // Copy
    impl<S: Serde, T: Copy> Copy for Base64Of<S, T> {}

    // Debug
    impl<S: Serde, T: Debug> Debug for Base64Of<S, T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            self.inner.fmt(f)
        }
    }

    // Display
    impl<S: Serde, T: Display> Display for Base64Of<S, T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            self.inner.fmt(f)
        }
    }

    // PartialEq
    impl<S: Serde, S2: Serde, T: PartialEq> PartialEq<Base64Of<S2, T>> for Base64Of<S, T> {
        fn eq(&self, other: &Base64Of<S2, T>) -> bool {
            self.inner.eq(&other.inner)
        }
    }

    impl<S: Serde, T: PartialEq> PartialEq<T> for Base64Of<S, T> {
        fn eq(&self, other: &T) -> bool {
            self.inner.eq(other)
        }
    }

    // Eq
    // This implementation is not possible because the `S: Ser` type parameter
    // shouldn't matter in the `PartialEq` implementation, but `Eq` demands
    // that Rhs is Self, and Rust doesn't recognize that the `PartialEq` impl
    // covers that case already. Basically it doesn't understand that S1 and S2
    // _can_ be the same type.
    //
    // impl<S: Serde, T: Eq> Eq for Base64Of<S, T> {}

    // PartialOrd
    impl<S: Serde, S2: Serde, T: PartialOrd> PartialOrd<Base64Of<S2, T>> for Base64Of<S, T> {
        fn partial_cmp(&self, other: &Base64Of<S2, T>) -> Option<Ordering> {
            self.inner.partial_cmp(&other.inner)
        }
    }

    impl<S: Serde, T: PartialOrd> PartialOrd<T> for Base64Of<S, T> {
        fn partial_cmp(&self, other: &T) -> Option<Ordering> {
            self.inner.partial_cmp(other)
        }
    }

    // Ord
    // This can not be implemented for the same reason that `Eq` can't be implemented.

    // Hash
    impl<S: Serde, T: Hash> Hash for Base64Of<S, T> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.inner.hash(state)
        }
    }

    // Default
    impl<S: Serde, T: Default> Default for Base64Of<S, T> {
        fn default() -> Self {
            Self {
                inner: T::default(),
                ser: PhantomData,
            }
        }
    }

    // JsonSchema
    impl<S: Serde, T: JsonSchema> JsonSchema for Base64Of<S, T> {
        fn schema_name() -> String {
            Binary::schema_name()
        }

        fn json_schema(gen: &mut SchemaGenerator) -> Schema {
            Binary::json_schema(gen)
        }
    }
}

#[cfg(test)]
mod test {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use cosmwasm_schema::schema_for;
    use cosmwasm_std::{Binary, StdResult};

    use crate::base64::Base64JsonOf;

    #[derive(Deserialize, Serialize, PartialEq, Debug, JsonSchema)]
    struct Foo {
        bar: String,
        baz: u32,
    }

    impl Foo {
        fn new() -> Self {
            Self {
                bar: String::from("some stuff"),
                baz: 234,
            }
        }
    }

    #[derive(Deserialize, Serialize, PartialEq, Debug, JsonSchema)]
    struct Wrapper {
        inner: Base64JsonOf<Foo>,
    }

    impl Wrapper {
        fn new() -> Self {
            Self {
                inner: Base64JsonOf::from(Foo::new()),
            }
        }
    }

    #[test]
    fn test_serialize() -> StdResult<()> {
        let serialized = cosmwasm_std::to_vec(&Base64JsonOf::from(Foo::new()))?;
        let serialized2 =
            cosmwasm_std::to_vec(&Binary::from(b"{\"bar\":\"some stuff\",\"baz\":234}"))?;
        assert_eq!(
            br#""eyJiYXIiOiJzb21lIHN0dWZmIiwiYmF6IjoyMzR9""#[..],
            serialized
        );
        assert_eq!(serialized, serialized2);

        let serialized3 = cosmwasm_std::to_vec(&Wrapper::new())?;
        assert_eq!(
            br#"{"inner":"eyJiYXIiOiJzb21lIHN0dWZmIiwiYmF6IjoyMzR9"}"#[..],
            serialized3
        );

        Ok(())
    }

    #[test]
    fn test_deserialize() -> StdResult<()> {
        let obj: Base64JsonOf<Foo> =
            cosmwasm_std::from_slice(&br#""eyJiYXIiOiJzb21lIHN0dWZmIiwiYmF6IjoyMzR9""#[..])?;
        assert_eq!(obj, Foo::new());

        let obj2: Wrapper = cosmwasm_std::from_slice(
            &br#"{"inner":"eyJiYXIiOiJzb21lIHN0dWZmIiwiYmF6IjoyMzR9"}"#[..],
        )?;
        assert_eq!(obj2, Wrapper::new());
        assert_eq!(obj2.inner, Foo::new());

        Ok(())
    }

    #[test]
    fn test_schema() {
        let schema = schema_for!(Wrapper);
        let pretty = serde_json::to_string_pretty(&schema).unwrap();
        println!("{}", pretty);
        println!("{:#?}", schema);

        assert_eq!(schema.schema.metadata.unwrap().title.unwrap(), "Wrapper");
        let object = schema.schema.object.unwrap();
        let required = object.required;
        assert_eq!(required.len(), 1);
        assert!(required.contains("inner"));

        // This checks that the schema sees the Base64Of field as a Binary object
        if let schemars::schema::Schema::Object(ref obj) = object.properties["inner"] {
            assert_eq!(obj.reference.as_ref().unwrap(), "#/definitions/Binary")
        } else {
            panic!("unexpected schema");
        }
    }
}
