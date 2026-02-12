use golden_schema::NodeId;
use golden_schema::{ColorRgba, ReferenceValue, Trigger, Value, Vec2, Vec3};

use crate::edits::Propagation;
use crate::engine::ProcessCtx;

pub type ParameterData = golden_schema::ParameterData;

pub trait ParameterValue: Sized {
    fn into_value(self) -> Value;
    fn from_value(value: &Value) -> Option<Self>;
}

pub struct ParameterHandle<T> {
    pub node_id: NodeId,
    _marker: std::marker::PhantomData<T>,
}

impl<T> ParameterHandle<T> {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> ParameterHandle<T>
where
    T: ParameterValue,
{
    pub fn get(&self, ctx: &ProcessCtx) -> Option<T> {
        ctx.read_param(self.node_id).and_then(T::from_value)
    }

    pub fn set(&self, ctx: &mut ProcessCtx, value: T) {
        ctx.set_param(self.node_id, value.into_value());
    }

    pub fn set_immediate(&self, ctx: &mut ProcessCtx, value: T) {
        ctx.set_param_with(self.node_id, value.into_value(), Propagation::Immediate);
    }

    pub fn set_next_tick(&self, ctx: &mut ProcessCtx, value: T) {
        ctx.set_param_with(self.node_id, value.into_value(), Propagation::NextTick);
    }
}

impl ParameterValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

impl ParameterValue for i64 {
    fn into_value(self) -> Value {
        Value::Int(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }
}

impl ParameterValue for f64 {
    fn into_value(self) -> Value {
        Value::Float(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Float(v) => Some(*v),
            _ => None,
        }
    }
}

impl ParameterValue for String {
    fn into_value(self) -> Value {
        Value::String(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::String(v) => Some(v.clone()),
            _ => None,
        }
    }
}

impl ParameterValue for Vec2 {
    fn into_value(self) -> Value {
        Value::Vec2(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Vec2(v) => Some(*v),
            _ => None,
        }
    }
}

impl ParameterValue for Vec3 {
    fn into_value(self) -> Value {
        Value::Vec3(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Vec3(v) => Some(*v),
            _ => None,
        }
    }
}

impl ParameterValue for ColorRgba {
    fn into_value(self) -> Value {
        Value::ColorRgba(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::ColorRgba(v) => Some(*v),
            _ => None,
        }
    }
}

impl ParameterValue for ReferenceValue {
    fn into_value(self) -> Value {
        Value::Reference(self)
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Reference(v) => Some(v.clone()),
            _ => None,
        }
    }
}

impl ParameterValue for Trigger {
    fn into_value(self) -> Value {
        Value::Trigger
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Trigger => Some(Trigger),
            _ => None,
        }
    }
}
