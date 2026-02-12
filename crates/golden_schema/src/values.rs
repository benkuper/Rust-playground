use core::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{EnumId, EnumVariantId, NodeId, NodeUuid};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceValue {
    pub uuid: NodeUuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_id: Option<NodeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    ColorRgba(ColorRgba),
    Trigger,
    Enum {
        enum_id: EnumId,
        variant: EnumVariantId,
    },
    Reference(ReferenceValue),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(v) => write!(f, "{v}"),
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "\"{v}\""),
            Value::Vec2(v) => write!(f, "Vec2({}, {})", v.x, v.y),
            Value::Vec3(v) => write!(f, "Vec3({}, {}, {})", v.x, v.y, v.z),
            Value::ColorRgba(v) => write!(f, "ColorRgba({}, {}, {}, {})", v.r, v.g, v.b, v.a),
            Value::Trigger => write!(f, "Trigger"),
            Value::Enum {
                enum_id,
                variant,
            } => write!(f, "Enum({:?}, {:?})", enum_id, variant),
            Value::Reference(reference) => write!(
                f,
                "Reference(uuid: {:?}, cached_id: {:?})",
                reference.uuid, reference.cached_id
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdatePolicy {
    Immediate,
    EndOfTick,
    NextTick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangePolicy {
    ValueChange,
    Always,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavePolicy {
    None,
    Delta,
    Full,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ValueConstraints {
    None,
    Int {
        min: Option<i64>,
        max: Option<i64>,
        clamp: bool,
        step: Option<i64>,
    },
    Float {
        min: Option<f64>,
        max: Option<f64>,
        clamp: bool,
        step: Option<f64>,
    },
    String {
        max_len: Option<usize>,
        pattern: Option<String>,
    },
    Enum {
        enum_id: EnumId,
        allowed: Vec<EnumVariantId>,
    },
    Reference {
        target: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParameterData {
    pub value: Value,
    pub default: Option<Value>,
    pub read_only: bool,
    pub update: UpdatePolicy,
    pub save: SavePolicy,
    pub change: ChangePolicy,
    pub constraints: ValueConstraints,
}
