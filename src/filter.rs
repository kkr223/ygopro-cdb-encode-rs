use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    Null,
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    Bool(bool),
    Text(String),
}

impl FilterValue {
    pub fn to_json(&self) -> JsonValue {
        match self {
            Self::Null => JsonValue::Null,
            Self::Integer(value) => JsonValue::from(*value),
            Self::Unsigned(value) => JsonValue::from(*value),
            Self::Float(value) => JsonValue::from(*value),
            Self::Bool(value) => JsonValue::from(*value),
            Self::Text(value) => JsonValue::from(value.clone()),
        }
    }
}

impl From<i32> for FilterValue {
    fn from(value: i32) -> Self {
        Self::Integer(i64::from(value))
    }
}

impl From<()> for FilterValue {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl From<i64> for FilterValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<u32> for FilterValue {
    fn from(value: u32) -> Self {
        Self::Unsigned(u64::from(value))
    }
}

impl From<u64> for FilterValue {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<f64> for FilterValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for FilterValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<&str> for FilterValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for FilterValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "value", rename_all = "camelCase")]
pub enum FilterCondition {
    Eq(FilterValue),
    NotEq(FilterValue),
    LessThan(FilterValue),
    LessThanOrEqual(FilterValue),
    MoreThan(FilterValue),
    MoreThanOrEqual(FilterValue),
    HasBit(u64),
    HasAllBits(u64),
    And(Vec<FilterCondition>),
    Or(Vec<FilterCondition>),
    Not(Box<FilterCondition>),
}

impl FilterCondition {
    pub fn eq(value: impl Into<FilterValue>) -> Self {
        Self::Eq(value.into())
    }

    pub fn less_than(value: impl Into<FilterValue>) -> Self {
        Self::LessThan(value.into())
    }

    pub fn less_than_or_equal(value: impl Into<FilterValue>) -> Self {
        Self::LessThanOrEqual(value.into())
    }

    pub fn more_than(value: impl Into<FilterValue>) -> Self {
        Self::MoreThan(value.into())
    }

    pub fn more_than_or_equal(value: impl Into<FilterValue>) -> Self {
        Self::MoreThanOrEqual(value.into())
    }

    pub fn has_bit(value: u64) -> Self {
        Self::HasBit(value)
    }

    pub fn has_all_bits(value: u64) -> Self {
        Self::HasAllBits(value)
    }

    pub fn and(list: impl Into<Vec<FilterCondition>>) -> Self {
        Self::And(list.into())
    }

    pub fn or(list: impl Into<Vec<FilterCondition>>) -> Self {
        Self::Or(list.into())
    }

    pub fn not(condition: FilterCondition) -> Self {
        Self::Not(Box::new(condition))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FindFilter {
    pub fields: BTreeMap<String, FilterCondition>,
}

impl FindFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, field: impl Into<String>, condition: FilterCondition) -> Self {
        self.fields.insert(field.into(), condition);
        self
    }
}

pub fn not(condition: FilterCondition) -> FilterCondition {
    FilterCondition::not(condition)
}

pub fn less_than(value: impl Into<FilterValue>) -> FilterCondition {
    FilterCondition::less_than(value)
}

pub fn more_than(value: impl Into<FilterValue>) -> FilterCondition {
    FilterCondition::more_than(value)
}

pub fn less_than_or_equal(value: impl Into<FilterValue>) -> FilterCondition {
    FilterCondition::less_than_or_equal(value)
}

pub fn more_than_or_equal(value: impl Into<FilterValue>) -> FilterCondition {
    FilterCondition::more_than_or_equal(value)
}

pub fn and<I>(values: I) -> FilterCondition
where
    I: IntoIterator<Item = FilterCondition>,
{
    FilterCondition::And(values.into_iter().collect())
}

pub fn or<I>(values: I) -> FilterCondition
where
    I: IntoIterator<Item = FilterCondition>,
{
    FilterCondition::Or(values.into_iter().collect())
}

pub fn has_bit(value: u64) -> FilterCondition {
    FilterCondition::has_bit(value)
}

pub fn has_all_bits(value: u64) -> FilterCondition {
    FilterCondition::has_all_bits(value)
}
