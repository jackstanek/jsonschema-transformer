use std::{collections::HashMap, sync::Arc};

use serde_json::{Value, json, Map};

#[derive(Debug)]
enum SchemaNode {
    List {
        items: Arc<SchemaNode>,
    },
    Object {
        properties: HashMap<String, SchemaNode>,
        additional_properties: Arc<SchemaNode>,
    },
    Number,
    String,
    Null,
    Bool,

    // Special schemas
    False, // Validation always fails
    True,  // Validation always succeeds
}

impl SchemaNode {
    fn object_node(schema: Map<String, Value>) -> Result<SchemaNode, SchemaErr> {
        for (key, value) in schema {
            match key.as_str() {
                "type" => if let Value::String(tyname) = value {
                    match tyname.as_str() {
                        "bool"   => 
                        "number" => 
                        "null" => 
                        "bool" => 
                        "bool" => 
                        "bool" => 
                    }
                }
            }    
        }
        todo!()
    }
}

#[derive(Debug)]
enum SchemaErr {
    InvalidSchema,
}

impl TryFrom<Value> for SchemaNode {
    type Error = SchemaErr;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        use SchemaErr::*;
        match value {
            Value::Bool(val) => Ok(if val {
                SchemaNode::True
            } else {
                SchemaNode::False
            }),
            Value::Object(schema) => Self::object_node(schema),
            _ => Err(InvalidSchema),
        }
    }
}

fn main() {
    let raw_schema = json!({
        "type": "bool"
    });
    println!("{:?}", SchemaNode::try_from(raw_schema));
}
