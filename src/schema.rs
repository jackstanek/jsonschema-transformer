use std::{
    cmp::Ordering,
    collections::BTreeMap,
    ops::{Add, AddAssign},
    sync::Arc,
};

use serde_json::Value;

/// Error while parsing a [`Schema`] from json. One of these errors will be returned
/// in the case that the json is not our case of valid.
#[derive(Debug)]
pub enum SchemaErr {
    InvalidSchema,
    ArrNeedsItems,
    ObjNeedsProperties,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ground {
    Num,
    Bool,
    String,
    Null,
}

/// Top-level schema representation. Num, Bool, String, and Null represent
/// schemas which match against those types of data. Arr and Obj are recursive
/// schemas; Arr's subschema matches against the items in the list, and Obj is a
/// map between the property names and their respective schemas. True and False
/// are trivial schemas which always or never validate, respectively.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Schema {
    Ground(Ground),
    Arr(Arc<Schema>),
    Obj(BTreeMap<Arc<String>, Arc<Schema>>),
    True,
    False,
}

/// Create a [`Schema`] from raw JSON.
#[macro_export]
macro_rules! schema {
    ($($v:tt)?) => {
        {
            $(
            let json_schema = serde_json::json!($v);
            super::Schema::try_from(&json_schema).unwrap()
            )?
        }
    };
}

impl From<bool> for Schema {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }
}

impl TryFrom<&Value> for Schema {
    type Error = SchemaErr;

    fn try_from(value: &Value) -> Result<Schema, Self::Error> {
        use SchemaErr::*;

        match value {
            Value::Bool(b) => Ok(Schema::from(*b)),
            Value::Object(obj) => {
                let ty = obj.get("type").ok_or(InvalidSchema)?;
                if let Value::String(tyname) = ty {
                    return match tyname.as_str() {
                        "number" => Ok(Self::num()),
                        "string" => Ok(Self::string()),
                        "boolean" => Ok(Self::bool()),
                        "null" => Ok(Self::null()),
                        "array" => {
                            return if let Some(item_type) = obj.get("items") {
                                let item_type = Self::try_from(item_type)?;
                                Ok(Schema::Arr(Arc::new(item_type)))
                            } else {
                                Err(ArrNeedsItems)
                            }
                        }
                        "object" => {
                            let props = obj.get("properties");
                            let mut subschemas = BTreeMap::new();
                            if let Some(Value::Object(props)) = props {
                                for (prop, subschema) in props.iter() {
                                    subschemas.insert(
                                        Arc::new(prop.clone()),
                                        Arc::new(Self::try_from(subschema)?),
                                    );
                                }
                                Ok(Schema::Obj(subschemas))
                            } else {
                                Err(ObjNeedsProperties)
                            }
                        }
                        _ => Err(InvalidSchema),
                    };
                }
                Err(InvalidSchema)
            }
            _ => Err(InvalidSchema),
        }
    }
}

impl Schema {
    fn num() -> Self {
        Self::Ground(Ground::Num)
    }

    fn bool() -> Self {
        Self::Ground(Ground::Bool)
    }

    fn string() -> Self {
        Self::Ground(Ground::String)
    }

    fn null() -> Self {
        Self::Ground(Ground::Null)
    }

}

// #[cfg(test)]
// mod tests {
//     use super::Schema;
//     use super::Schema::*;
//     use crate::schema;

//     #[test]
//     fn test_same_base_type_edit_dist() {
//         let v1 = Schema::bool();
//         let v2 = Schema::bool();
//         assert_eq!(v1.edit_distance(&v2), Nat(0));
//     }

//     #[test]
//     fn test_base_type_edit_dist() {
//         let v1 = Schema::bool();
//         let v2 = Schema::num();
//         assert_eq!(v1.edit_distance(&v2), Nat(1));
//     }

//     #[test]
//     fn test_arr_type_edit_dist() {
//         let v1 = schema!({
//             "type": "array",
//             "items": {
//                 "type": "boolean"
//             }
//         });
//         let v2 = schema!({
//             "type": "array",
//             "items": {
//                 "type": "number"
//             }
//         });
//         assert_eq!(v1.edit_distance(&v2), Nat(1))
//     }

//     #[test]
//     fn test_flat_obj_typ_edit_dist() {
//         let v1 = schema!({
//             "type": "object",
//             "properties": {
//                 "foo": {
//                     "type": "number"
//                 },
//                 "bar": {
//                     "type": "boolean"
//                 }
//             }
//         });
//         let v2 = schema!({
//             "type": "object",
//             "properties": {
//                 "foo": {
//                     "type": "string"
//                 },
//                 "bar": {
//                     "type": "string"
//                 }
//             }
//         });
//         assert_eq!(v1.edit_distance(&v2), Nat(2))
//     }

//     // change path to wherever your project is located
//     #[test]
//     fn test_open_file() {
//         let path = "/Users/dkillough/Desktop/gradschool/jsonschema-transformer/schemas/simple.json";
//         let file = std::fs::read_to_string(path).unwrap();
//         let json_schema: serde_json::Value = serde_json::from_str(&file).unwrap();
//         let testjson = schema!(
//             {
//                 "type": "object",
//                 "properties": {
//                   "nullValue": {
//                     "type": "null"
//                   },
//                   "booleanValue": {
//                     "type": "boolean"
//                   },
//                   "objectValue": {
//                     "type": "object",
//                     "properties": {
//                         "foo": {
//                             "type": "string"
//                         },
//                     }
//                   },
//                   "arrayValue": {
//                     "type": "array",
//                     "items": {
//                         "type": "string"
//                     }
//                   },
//                   "numberValue": {
//                     "type": "number"
//                   },
//                   "stringValue": {
//                     "type": "string"
//                   }
//                 },
//                 "required": [
//                   "nullValue",
//                   "booleanValue",
//                   "objectValue",
//                   "arrayValue",
//                   "numberValue",
//                   "stringValue"
//                 ],
//                 "additionalProperties": false
//               }
//         );
//         assert_eq!(testjson, super::Schema::try_from(&json_schema).unwrap());
//     }
// }
