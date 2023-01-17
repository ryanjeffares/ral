use std::{
    fmt,
    mem::ManuallyDrop,
    ops::{Add, Div, Mul, Sub},
    rc::Rc,
};

use crate::audio::audio_buffer::AudioBuffer;

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
    audio: ManuallyDrop<Rc<AudioBuffer>>,
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

    pub fn audio(value: AudioBuffer) -> Self {
        Value {
            value_type: ValueType::Audio,
            value: Data {
                audio: ManuallyDrop::<Rc<AudioBuffer>>::new(Rc::new(value)),
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

    pub fn get_audio(&self) -> &AudioBuffer {
        unsafe { self.value.audio.as_ref() }
    }

    pub fn get_string(&self) -> &String {
        unsafe { self.value.string.as_ref() }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.value_type != other.value_type {
            false
        } else {
            match self.value_type {
                ValueType::Audio => false, // TODO
                ValueType::Int => unsafe { self.value.int == other.value.int },
                ValueType::Float => unsafe { self.value.float == other.value.float },
                ValueType::String => unsafe {
                    self.value.string.as_ref() == other.value.string.as_ref()
                },
            }
        }
    }
}

impl Mul for Value {
    type Output = Value;
    // TODO: allow different types - audio * float, string * int etc...
    fn mul(self, rhs: Self) -> Self::Output {
        match self.value_type {
            ValueType::Audio => {
                let mut buffer = self.get_audio().clone();
                match rhs.value_type {
                    ValueType::Audio => buffer.multiply_by(rhs.get_audio()),
                    ValueType::Float => buffer.apply_gain(rhs.get_float()),
                    ValueType::Int => buffer.apply_gain(rhs.get_int() as f32),
                    _ => unreachable!(),
                }
                Value::audio(buffer)
            }
            ValueType::Int => Value::int(
                self.get_int()
                    * match rhs.value_type {
                        ValueType::Int => rhs.get_int(),
                        ValueType::Float => rhs.get_float() as i64,
                        _ => unreachable!(),
                    },
            ),
            ValueType::Float => Value::float(
                self.get_float()
                    * match rhs.value_type {
                        ValueType::Int => rhs.get_int() as f32,
                        ValueType::Float => rhs.get_float(),
                        _ => unreachable!(),
                    },
            ),
            ValueType::String => unreachable!(),
        }
    }
}

impl Div for Value {
    type Output = Value;
    fn div(self, rhs: Self) -> Self::Output {
        match self.value_type {
            ValueType::Audio => unreachable!(),
            ValueType::Int => Value::int(
                self.get_int()
                    / match rhs.value_type {
                        ValueType::Int => rhs.get_int(),
                        ValueType::Float => rhs.get_float() as i64,
                        _ => unreachable!(),
                    },
            ),
            ValueType::Float => Value::float(
                self.get_float()
                    / match rhs.value_type {
                        ValueType::Int => rhs.get_int() as f32,
                        ValueType::Float => rhs.get_float(),
                        _ => unreachable!(),
                    },
            ),
            ValueType::String => unreachable!(),
        }
    }
}

impl Add for Value {
    type Output = Value;
    fn add(self, rhs: Self) -> Self::Output {
        match self.value_type {
            ValueType::Audio => {
                let mut buffer = self.get_audio().clone();
                match rhs.value_type {
                    ValueType::Audio => buffer.add_from(rhs.get_audio()),
                    ValueType::Float => buffer.apply_add(rhs.get_float()),
                    ValueType::Int => buffer.apply_add(rhs.get_int() as f32),
                    _ => unreachable!(),
                }
                Value::audio(buffer)
            }
            ValueType::Int => Value::int(
                self.get_int()
                    + match rhs.value_type {
                        ValueType::Int => rhs.get_int(),
                        ValueType::Float => rhs.get_float() as i64,
                        _ => unreachable!(),
                    },
            ),
            ValueType::Float => Value::float(
                self.get_float()
                    + match rhs.value_type {
                        ValueType::Int => rhs.get_int() as f32,
                        ValueType::Float => rhs.get_float(),
                        _ => unreachable!(),
                    },
            ),
            ValueType::String => Value::string(self.get_string().to_owned() + rhs.get_string()),
        }
    }
}

impl Sub for Value {
    type Output = Value;
    fn sub(self, rhs: Self) -> Self::Output {
        match self.value_type {
            ValueType::Audio => {
                let mut buffer = self.get_audio().clone();
                match rhs.value_type {
                    ValueType::Audio => buffer.subtract_from(rhs.get_audio()),
                    ValueType::Float => buffer.apply_add(-rhs.get_float()),
                    ValueType::Int => buffer.apply_add(-rhs.get_int() as f32),
                    _ => unreachable!(),
                }
                Value::audio(buffer)
            }
            ValueType::Int => Value::int(
                self.get_int()
                    - match rhs.value_type {
                        ValueType::Float => rhs.get_float() as i64,
                        ValueType::Int => rhs.get_int(),
                        _ => unreachable!(),
                    },
            ),
            ValueType::Float => Value::float(
                self.get_float()
                    - match rhs.value_type {
                        ValueType::Float => rhs.get_float(),
                        ValueType::Int => rhs.get_int() as f32,
                        _ => unreachable!(),
                    },
            ),
            ValueType::String => unreachable!(),
        }
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
                    write!(f, "{:?}", self.value.audio.as_ref())
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
