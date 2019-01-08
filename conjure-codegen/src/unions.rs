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
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::iter;

use crate::context::Context;
use crate::types::UnionDefinition;

pub fn generate(ctx: &Context, def: &UnionDefinition) -> TokenStream {
    let conjure_types = ctx.conjure_path();
    let enum_ = generate_enum(ctx, def);
    let deserialize = generate_deserialize(ctx, def);
    let variant = generate_variant(ctx, def);
    let unknown = generate_unknown(ctx, def);

    quote! {
        use #conjure_types::serde::{ser, de};
        use #conjure_types::serde::ser::SerializeMap as SerializeMap_;
        use #conjure_types::private::{UnionField_, UnionTypeField_};
        use std::fmt;

        #enum_
        #deserialize
        #variant
        #unknown
    }
}

fn variants(ctx: &Context, def: &UnionDefinition) -> Vec<Ident> {
    def.union_()
        .iter()
        .map(|f| ctx.type_name(f.field_name()))
        .collect()
}

fn unknown(ctx: &Context, def: &UnionDefinition) -> TokenStream {
    if variants(ctx, def).iter().any(|f| f == "Unknown") {
        quote!(Unknown_)
    } else {
        quote!(Unknown)
    }
}

fn generate_enum(ctx: &Context, def: &UnionDefinition) -> TokenStream {
    let name = ctx.type_name(def.type_name().name());
    let result = ctx.result_ident(def.type_name());
    let some = ctx.some_ident(def.type_name());

    let mut derives = vec!["Debug", "Clone", "PartialEq", "PartialOrd"];
    if !def.union_().iter().any(|v| ctx.has_double(v.type_())) {
        derives.push("Eq");
        derives.push("Ord");
        derives.push("Hash");
    }
    let derives = derives.iter().map(|s| s.parse::<TokenStream>().unwrap());

    let docs = def.union_().iter().map(|f| ctx.docs(f.docs()));

    let variants = &variants(ctx, def);

    let types = &def
        .union_()
        .iter()
        .map(|f| ctx.boxed_rust_type(def.type_name(), f.type_()))
        .collect::<Vec<_>>();

    let unknown = unknown(ctx, def);
    let unknown_variant = if ctx.exhaustive() {
        quote!()
    } else {
        quote! {
            /// An unknown variant.
            #unknown(#unknown),
        }
    };

    let serialize_unknown = if ctx.exhaustive() {
        quote!()
    } else {
        quote! {
            #name::#unknown(value) => {
                map.serialize_entry(&"type", &value.type_)?;
                map.serialize_entry(&value.type_, &value.value)?;
            }
        }
    };

    let variant_strs = &def
        .union_()
        .iter()
        .map(|f| &f.field_name().0)
        .collect::<Vec<_>>();
    let variant_strs2 = variant_strs;
    let name_repeat = iter::repeat(&name);

    quote! {
        #[derive(#(#derives),*)]
        pub enum #name {
            #(
                #docs
                #variants(#types),
            )*
            #unknown_variant
        }

        impl ser::Serialize for #name {
            fn serialize<S_>(&self, s: S_) -> #result<S_::Ok, S_::Error>
            where
                S_: ser::Serializer
            {
                let mut map = s.serialize_map(#some(2))?;

                match self {
                    #(
                        #name_repeat::#variants(value) => {
                            map.serialize_entry(&"type", &#variant_strs)?;
                            map.serialize_entry(&#variant_strs2, value)?;
                        }
                    )*
                    #serialize_unknown
                }

                map.end()
            }
        }
    }
}

fn generate_deserialize(ctx: &Context, def: &UnionDefinition) -> TokenStream {
    let name = ctx.type_name(def.type_name().name());
    let result = ctx.result_ident(def.type_name());

    let expecting = format!("union {}", name);

    let some = ctx.some_ident(def.type_name());

    let variants = &variants(ctx, def);
    let variants2 = variants;
    let variants3 = variants;

    let name_repeat = iter::repeat(&name);
    let some_repeat = iter::repeat(&some);

    let unknown = unknown(ctx, def);

    let err = ctx.err_ident(def.type_name());

    let unknown_match1 = if ctx.exhaustive() {
        quote!()
    } else {
        quote! {
            (Variant_::#unknown(type_), #some(Variant_::#unknown(b))) => {
                if type_ == b {
                    let value = map.next_value()?;
                    #name::#unknown(#unknown { type_, value })
                } else {
                    return #err(de::Error::invalid_value(de::Unexpected::Str(&type_), &&*b))
                }
            }
        }
    };

    let none = ctx.none_ident(def.type_name());

    let name_repeat2 = iter::repeat(&name);

    let unknown_match2 = if ctx.exhaustive() {
        quote!()
    } else {
        quote! {
            Variant_::#unknown(type_) => {
                let value = map.next_value()?;
                #name::#unknown(#unknown { type_: type_.clone(), value })
            }
        }
    };

    let ok = ctx.ok_ident(def.type_name());

    quote! {
        impl<'de> de::Deserialize<'de> for #name {
            fn deserialize<D_>(d: D_) -> #result<#name, D_::Error>
            where
                D_: de::Deserializer<'de>
            {
                d.deserialize_map(Visitor_)
            }
        }

        struct Visitor_;

        impl<'de> de::Visitor<'de> for Visitor_ {
            type Value = #name;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str(#expecting)
            }

            fn visit_map<A_>(self, mut map: A_) -> #result<#name, A_::Error>
            where
                A_: de::MapAccess<'de>
            {
                let v = match map.next_key::<UnionField_<Variant_>>()? {
                    #some(UnionField_::Type) => {
                        let variant = map.next_value()?;
                        let key = map.next_key()?;
                        match (variant, key) {
                            #(
                                (Variant_::#variants, #some_repeat(Variant_::#variants2)) => {
                                    let value = map.next_value()?;
                                    #name_repeat::#variants3(value)
                                }
                            )*
                            #unknown_match1
                            (variant, #some(key)) => {
                                return #err(
                                    de::Error::invalid_value(de::Unexpected::Str(key.as_str()), &variant.as_str()),
                                );
                            }
                            (variant, #none) => return #err(de::Error::missing_field(variant.as_str())),
                        }
                    }
                    #some(UnionField_::Value(variant)) => {
                        let value = match &variant {
                            #(
                                Variant_::#variants => {
                                    let value = map.next_value()?;
                                    #name_repeat2::#variants2(value)
                                }
                            )*
                            #unknown_match2
                        };

                        if map.next_key::<UnionTypeField_>()?.is_none() {
                            return #err(de::Error::missing_field("type"));
                        }

                        let type_variant = map.next_value::<Variant_>()?;
                        if variant != type_variant {
                            return #err(
                                de::Error::invalid_value(de::Unexpected::Str(type_variant.as_str()), &variant.as_str()),
                            );
                        }

                        value
                    }
                    #none => return #err(de::Error::missing_field("type")),
                };

                if map.next_key::<UnionField_<Variant_>>()?.is_some() {
                    return #err(de::Error::invalid_length(3, &"type and value fields"));
                }

                #ok(v)
            }
        }
    }
}

fn generate_variant(ctx: &Context, def: &UnionDefinition) -> TokenStream {
    let variants = &variants(ctx, def);

    let unknown = unknown(ctx, def);

    let unknown_variant = if ctx.exhaustive() {
        quote!()
    } else {
        let string = ctx.string_ident(def.type_name());
        quote!(#unknown(#string))
    };

    let variant_strs = &def
        .union_()
        .iter()
        .map(|f| &f.field_name().0)
        .collect::<Vec<_>>();

    let unknown_as_str = if ctx.exhaustive() {
        quote!()
    } else {
        quote! {
            Variant_::#unknown(_) => "unknown variant",
        }
    };

    let result = ctx.result_ident(def.type_name());

    let unknown_de_visit_str = if ctx.exhaustive() {
        let err = ctx.err_ident(def.type_name());
        quote! {
            value => return #err(de::Error::unknown_variant(value, &[#(#variant_strs, )*])),
        }
    } else {
        quote! {
            value => Variant_::#unknown(value.to_string()),
        }
    };

    let ok = ctx.ok_ident(def.type_name());

    quote! {
        #[derive(PartialEq)]
        enum Variant_ {
            #(#variants,)*
            #unknown_variant
        }

        impl Variant_ {
            fn as_str(&self) -> &'static str {
                match self {
                    #(
                        Variant_::#variants => #variant_strs,
                    )*
                    #unknown_as_str
                }
            }
        }

        impl<'de> de::Deserialize<'de> for Variant_ {
            fn deserialize<D_>(d: D_) -> #result<Variant_, D_::Error>
            where
                D_: de::Deserializer<'de>
            {
                d.deserialize_str(VariantVisitor_)
            }
        }

        struct VariantVisitor_;

        impl<'de> de::Visitor<'de> for VariantVisitor_ {
            type Value = Variant_;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str("string")
            }

            fn visit_str<E_>(self, value: &str) -> #result<Variant_, E_>
            where
                E_: de::Error,
            {
                let v = match value {
                    #(
                        #variant_strs => Variant_::#variants,
                    )*
                    #unknown_de_visit_str
                };

                #ok(v)
            }
        }
    }
}

fn generate_unknown(ctx: &Context, def: &UnionDefinition) -> TokenStream {
    let conjure_types = ctx.conjure_path();

    if ctx.exhaustive() {
        return quote!();
    }

    let doc = format!(
        "An unknown variant of the {} union.",
        ctx.type_name(def.type_name().name())
    );

    let unknown = unknown(ctx, def);

    quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct #unknown {
            type_: String,
            value: #conjure_types::Value,
        }

        impl #unknown {
            /// Returns the unknown variant's type name.
            #[inline]
            pub fn type_(&self) -> &str {
                &self.type_
            }
        }
    }
}