use crate::{audio::components::component::Component, runtime::instrument::VariableType};

pub trait Generator<const ARG_COUNT: usize>: Component {
    const INPUT_TYPES: [VariableType; ARG_COUNT];
    const OUTPUT_TYPE: VariableType;
}
