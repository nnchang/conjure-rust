// Copyright 2018 Palantir Technologies, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use serde::de;
use std::fmt;

use crate::ClientDeserializer;

/// A serde deserializer appropriate for use by Conjure servers.
///
/// Specifically, the f32 and f64 types can be deserialized from the strings `"Infinity"`, `"-Infinity"`, and `"NaN"`,
/// and bytes are deserialized from base64 encoded strings. Unknown object fields trigger errors.
pub struct ServerDeserializer<T> {
    deserializer: ClientDeserializer<T>,
}

impl<'de, T> ServerDeserializer<T>
where
    T: de::Deserializer<'de>,
{
    pub fn new(deserializer: T) -> ServerDeserializer<T> {
        ServerDeserializer {
            deserializer: ClientDeserializer::new(deserializer),
        }
    }
}

macro_rules! delegate_deserialize {
    ($($method:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, T::Error>
            where
                V: de::Visitor<'de>
            {
                self.deserializer.$method(Visitor { visitor })
            }
        )*
    }
}

impl<'de, T> de::Deserializer<'de> for ServerDeserializer<T>
where
    T: de::Deserializer<'de>,
{
    type Error = T::Error;

    delegate_deserialize!(
        deserialize_any,
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_option,
        deserialize_unit,
        deserialize_seq,
        deserialize_map,
        deserialize_identifier,
        deserialize_ignored_any,
        deserialize_i128,
        deserialize_u128,
    );

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_unit_struct(name, Visitor { visitor })
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_newtype_struct(name, Visitor { visitor })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_tuple(len, Visitor { visitor })
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_tuple_struct(name, len, Visitor { visitor })
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_struct(name, fields, StructVisitor { visitor, fields })
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_enum(name, variants, Visitor { visitor })
    }

    fn is_human_readable(&self) -> bool {
        self.deserializer.is_human_readable()
    }
}

struct Visitor<T> {
    visitor: T,
}

macro_rules! delegate_visit {
    ($($method:ident = $ty:ty,)*) => {
        $(
            fn $method<E>(self, v: $ty) -> Result<T::Value, E>
            where
                E: de::Error,
            {
                self.visitor.$method(v)
            }
        )*
    };
}

impl<'de, T> de::Visitor<'de> for Visitor<T>
where
    T: de::Visitor<'de>,
{
    type Value = T::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.visitor.expecting(formatter)
    }

    delegate_visit!(
        visit_bool = bool,
        visit_i8 = i8,
        visit_i16 = i16,
        visit_i32 = i32,
        visit_i64 = i64,
        visit_i128 = i128,
        visit_u8 = u8,
        visit_u16 = u16,
        visit_u32 = u32,
        visit_u64 = u64,
        visit_u128 = u128,
        visit_f32 = f32,
        visit_f64 = f64,
        visit_char = char,
        visit_str = &str,
        visit_borrowed_str = &'de str,
        visit_string = String,
        visit_bytes = &[u8],
        visit_borrowed_bytes = &'de [u8],
        visit_byte_buf = Vec<u8>,
    );

    fn visit_none<E>(self) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor
            .visit_some(ServerDeserializer::new(deserializer))
    }

    fn visit_unit<E>(self) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_unit()
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor
            .visit_newtype_struct(ServerDeserializer::new(deserializer))
    }

    fn visit_seq<A>(self, seq: A) -> Result<T::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        self.visitor.visit_seq(SeqAccess(seq))
    }

    fn visit_map<A>(self, map: A) -> Result<T::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        self.visitor.visit_map(MapAccess(map))
    }

    fn visit_enum<A>(self, data: A) -> Result<T::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        self.visitor.visit_enum(EnumAccess(data))
    }
}

struct StructVisitor<T> {
    visitor: T,
    fields: &'static [&'static str],
}

impl<'de, T> de::Visitor<'de> for StructVisitor<T>
where
    T: de::Visitor<'de>,
{
    type Value = T::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.visitor.expecting(formatter)
    }

    delegate_visit!(
        visit_bool = bool,
        visit_i8 = i8,
        visit_i16 = i16,
        visit_i32 = i32,
        visit_i64 = i64,
        visit_i128 = i128,
        visit_u8 = u8,
        visit_u16 = u16,
        visit_u32 = u32,
        visit_u64 = u64,
        visit_u128 = u128,
        visit_f32 = f32,
        visit_f64 = f64,
        visit_char = char,
        visit_str = &str,
        visit_borrowed_str = &'de str,
        visit_string = String,
        visit_bytes = &[u8],
        visit_borrowed_bytes = &'de [u8],
        visit_byte_buf = Vec<u8>,
    );

    fn visit_none<E>(self) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor
            .visit_some(ServerDeserializer::new(deserializer))
    }

    fn visit_unit<E>(self) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_unit()
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor
            .visit_newtype_struct(ServerDeserializer::new(deserializer))
    }

    fn visit_seq<A>(self, seq: A) -> Result<T::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        self.visitor.visit_seq(SeqAccess(seq))
    }

    fn visit_map<A>(self, map: A) -> Result<T::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        self.visitor.visit_map(StructMapAccess {
            map,
            fields: self.fields,
            key: None,
        })
    }

    fn visit_enum<A>(self, data: A) -> Result<T::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        self.visitor.visit_enum(EnumAccess(data))
    }
}

struct SeqAccess<T>(T);

impl<'de, T> de::SeqAccess<'de> for SeqAccess<T>
where
    T: de::SeqAccess<'de>,
{
    type Error = T::Error;

    fn next_element_seed<U>(&mut self, seed: U) -> Result<Option<U::Value>, T::Error>
    where
        U: de::DeserializeSeed<'de>,
    {
        self.0.next_element_seed(DeserializeSeed(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.0.size_hint()
    }
}

struct MapAccess<T>(T);

impl<'de, T> de::MapAccess<'de> for MapAccess<T>
where
    T: de::MapAccess<'de>,
{
    type Error = T::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, T::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.0.next_key_seed(DeserializeSeed(seed))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, T::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.0.next_value_seed(DeserializeSeed(seed))
    }

    #[allow(clippy::type_complexity)]
    fn next_entry_seed<K, V>(
        &mut self,
        kseed: K,
        vseed: V,
    ) -> Result<Option<(K::Value, V::Value)>, T::Error>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        self.0
            .next_entry_seed(DeserializeSeed(kseed), DeserializeSeed(vseed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.0.size_hint()
    }
}

struct StructMapAccess<T> {
    map: T,
    fields: &'static [&'static str],
    key: Option<String>,
}

impl<'de, T> de::MapAccess<'de> for StructMapAccess<T>
where
    T: de::MapAccess<'de>,
{
    type Error = T::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, T::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.key = None;
        self.map.next_key_seed(KeyDeserializeSeed {
            seed,
            key: &mut self.key,
        })
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, T::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.map.next_value_seed(ValueDeserializeSeed {
            seed,
            fields: self.fields,
            key: &self.key,
        })
    }

    fn size_hint(&self) -> Option<usize> {
        self.map.size_hint()
    }
}

struct EnumAccess<T>(T);

impl<'de, T> de::EnumAccess<'de> for EnumAccess<T>
where
    T: de::EnumAccess<'de>,
{
    type Error = T::Error;
    type Variant = VariantAccess<T::Variant>;

    #[allow(clippy::type_complexity)]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantAccess<T::Variant>), T::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.0
            .variant_seed(DeserializeSeed(seed))
            .map(|(value, variant)| (value, VariantAccess(variant)))
    }
}

struct VariantAccess<T>(T);

impl<'de, T> de::VariantAccess<'de> for VariantAccess<T>
where
    T: de::VariantAccess<'de>,
{
    type Error = T::Error;

    fn unit_variant(self) -> Result<(), T::Error> {
        self.0.unit_variant()
    }

    fn newtype_variant_seed<U>(self, seed: U) -> Result<U::Value, T::Error>
    where
        U: de::DeserializeSeed<'de>,
    {
        self.0.newtype_variant_seed(DeserializeSeed(seed))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.0.tuple_variant(len, Visitor { visitor })
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.0.struct_variant(fields, Visitor { visitor })
    }
}

struct DeserializeSeed<T>(T);

impl<'de, T> de::DeserializeSeed<'de> for DeserializeSeed<T>
where
    T: de::DeserializeSeed<'de>,
{
    type Value = T::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.0.deserialize(ServerDeserializer::new(deserializer))
    }
}

struct KeyDeserializeSeed<'a, T> {
    seed: T,
    key: &'a mut Option<String>,
}

impl<'de, 'a, T> de::DeserializeSeed<'de> for KeyDeserializeSeed<'a, T>
where
    T: de::DeserializeSeed<'de>,
{
    type Value = T::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.seed.deserialize(KeyDeserializer {
            deserializer,
            key: self.key,
        })
    }
}

struct KeyDeserializer<'a, T> {
    deserializer: T,
    key: &'a mut Option<String>,
}

macro_rules! delegate_key_deserialize {
    ($($method:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, T::Error>
            where
                V: de::Visitor<'de>
            {
                self.deserializer.$method(KeyVisitor { visitor, key: self.key })
            }
        )*
    }
}

impl<'de, 'a, T> de::Deserializer<'de> for KeyDeserializer<'a, T>
where
    T: de::Deserializer<'de>,
{
    type Error = T::Error;

    delegate_key_deserialize!(
        deserialize_any,
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_option,
        deserialize_unit,
        deserialize_seq,
        deserialize_map,
        deserialize_identifier,
        deserialize_ignored_any,
        deserialize_i128,
        deserialize_u128,
    );

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer.deserialize_unit_struct(
            name,
            KeyVisitor {
                visitor,
                key: self.key,
            },
        )
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer.deserialize_newtype_struct(
            name,
            KeyVisitor {
                visitor,
                key: self.key,
            },
        )
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer.deserialize_tuple(
            len,
            KeyVisitor {
                visitor,
                key: self.key,
            },
        )
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer.deserialize_tuple_struct(
            name,
            len,
            KeyVisitor {
                visitor,
                key: self.key,
            },
        )
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        // FIXME this is a bit awkward...
        self.deserializer
            .deserialize_struct(name, fields, StructVisitor { visitor, fields })
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer.deserialize_enum(
            name,
            variants,
            KeyVisitor {
                visitor,
                key: self.key,
            },
        )
    }

    fn is_human_readable(&self) -> bool {
        self.deserializer.is_human_readable()
    }
}

struct KeyVisitor<'a, T> {
    visitor: T,
    key: &'a mut Option<String>,
}

impl<'de, 'a, T> de::Visitor<'de> for KeyVisitor<'a, T>
where
    T: de::Visitor<'de>,
{
    type Value = T::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.visitor.expecting(formatter)
    }

    delegate_visit!(
        visit_bool = bool,
        visit_i8 = i8,
        visit_i16 = i16,
        visit_i32 = i32,
        visit_i64 = i64,
        visit_i128 = i128,
        visit_u8 = u8,
        visit_u16 = u16,
        visit_u32 = u32,
        visit_u64 = u64,
        visit_u128 = u128,
        visit_f32 = f32,
        visit_f64 = f64,
        visit_char = char,
        visit_bytes = &[u8],
        visit_borrowed_bytes = &'de [u8],
        visit_byte_buf = Vec<u8>,
    );

    fn visit_str<E>(self, value: &str) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        *self.key = Some(value.to_string());
        self.visitor.visit_str(value)
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        *self.key = Some(value.to_string());
        self.visitor.visit_borrowed_str(value)
    }

    fn visit_string<E>(self, value: String) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        *self.key = Some(value.clone());
        self.visitor.visit_string(value)
    }

    fn visit_none<E>(self) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor
            .visit_some(ServerDeserializer::new(deserializer))
    }

    fn visit_unit<E>(self) -> Result<T::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_unit()
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor
            .visit_newtype_struct(ServerDeserializer::new(deserializer))
    }

    fn visit_seq<A>(self, seq: A) -> Result<T::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        self.visitor.visit_seq(SeqAccess(seq))
    }

    fn visit_map<A>(self, map: A) -> Result<T::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        self.visitor.visit_map(MapAccess(map))
    }

    fn visit_enum<A>(self, data: A) -> Result<T::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        self.visitor.visit_enum(EnumAccess(data))
    }
}

struct ValueDeserializeSeed<'a, T> {
    seed: T,
    fields: &'static [&'static str],
    key: &'a Option<String>,
}

impl<'de, 'a, T> de::DeserializeSeed<'de> for ValueDeserializeSeed<'a, T>
where
    T: de::DeserializeSeed<'de>,
{
    type Value = T::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<T::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.seed.deserialize(ValueDeserializer {
            deserializer,
            fields: self.fields,
            key: self.key,
        })
    }
}

struct ValueDeserializer<'a, T> {
    deserializer: T,
    fields: &'static [&'static str],
    key: &'a Option<String>,
}

impl<'de, 'a, T> de::Deserializer<'de> for ValueDeserializer<'a, T>
where
    T: de::Deserializer<'de>,
{
    type Error = T::Error;

    delegate_deserialize!(
        deserialize_any,
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_option,
        deserialize_unit,
        deserialize_seq,
        deserialize_map,
        deserialize_identifier,
        deserialize_i128,
        deserialize_u128,
    );

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_unit_struct(name, Visitor { visitor })
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_newtype_struct(name, Visitor { visitor })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_tuple(len, Visitor { visitor })
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_tuple_struct(name, len, Visitor { visitor })
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_struct(name, fields, StructVisitor { visitor, fields })
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer
            .deserialize_enum(name, variants, Visitor { visitor })
    }

    fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, T::Error>
    where
        V: de::Visitor<'de>,
    {
        let key = match self.key {
            Some(key) => &**key,
            None => "<unknown>",
        };

        Err(de::Error::unknown_field(key, self.fields))
    }

    fn is_human_readable(&self) -> bool {
        self.deserializer.is_human_readable()
    }
}