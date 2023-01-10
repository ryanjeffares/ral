#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Op {
    AssignLocal,
    AssignMember,
    DeclareLocal,
    LoadArg,
    LoadConstant,
    LoadLocal,
    LoadMember,
    Print,
    PrintEmpty,
    PrintLn,
    PrintLnEmpty,
}
