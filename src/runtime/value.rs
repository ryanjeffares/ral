use std::{fmt, mem::ManuallyDrop};

use crate::utils::number_array::NumberArray;

pub struct Value {
    value_type: ValueType,
    value: Data,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueType {
    Audio,
    Int,
    Float,
    String,
}

union Data {
    int: i64,
    float: f32,
    string: ManuallyDrop<Box<String>>,
    audio: ManuallyDrop<Box<NumberArray<f32>>>,
}

impl Value {
    pub fn float(value: f32) -> Self {
        Value {
            value_type: ValueType::Float,
            value: Data { float: value },
        }
    }

    pub fn int(value: i64) -> Self {
        Value {
            value_type: ValueType::Int,
            value: Data { int: value },
        }
    }

    pub fn string(value: String) -> Self {
        Value {
            value_type: ValueType::String,
            value: Data {
                string: ManuallyDrop::<Box<String>>::new(Box::new(value)),
            },
        }
    }

    pub fn audio(value: NumberArray<f32>) -> Self {
        Value {
            value_type: ValueType::Audio,
            value: Data {
                audio: ManuallyDrop::<Box<NumberArray<f32>>>::new(Box::new(value)),
            },
        }
    }

    pub fn value_type(&self) -> ValueType {
        self.value_type
    }

    pub fn get_int(&self) -> i64 {
        unsafe { self.value.int }
    }

    pub fn get_float(&self) -> f32 {
        unsafe { self.value.float }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        unsafe {
            match self.value_type {
                ValueType::Audio => ManuallyDrop::drop(&mut self.value.audio),
                ValueType::String => ManuallyDrop::drop(&mut self.value.string),
                _ => (),
            }
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        unsafe {
            Value {
                value_type: self.value_type,
                value: match self.value_type {
                    ValueType::Audio => Data {
                        audio: self.value.audio.clone(),
                    },
                    ValueType::Int => Data {
                        int: self.value.int,
                    },
                    ValueType::Float => Data {
                        float: self.value.float,
                    },
                    ValueType::String => Data {
                        string: self.value.string.clone(),
                    },
                },
            }
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value {
            value_type: ValueType::Int,
            value: Data { int: 0 },
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.value_type {
                ValueType::Audio => {
                    write!(f, "{:?}", self.value.audio)
                }
                ValueType::Int => fmt::Display::fmt(&self.value.int, f),
                ValueType::Float => fmt::Display::fmt(&self.value.float, f),
                ValueType::String => fmt::Display::fmt(&*self.value.string, f),
            }
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.value_type {
                ValueType::Audio => fmt::Debug::fmt(&*self.value.audio, f),
                ValueType::Int => fmt::Debug::fmt(&self.value.int, f),
                ValueType::Float => fmt::Debug::fmt(&self.value.float, f),
                ValueType::String => fmt::Debug::fmt(&*self.value.string, f),
            }
        }
    }
}
