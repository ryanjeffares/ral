use super::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Add,
    AssignLocal(usize),
    AssignMember(usize),
    CallComponent(usize),
    DeclareLocal,
    Divide,
    LoadArg(usize),
    LoadConstant(Value),
    LoadLocal(usize),
    LoadMember(usize),
    Multiply,
    Output,
    Print,
    PrintEmpty,
    PrintLn,
    PrintLnEmpty,
    Subtract,
}
