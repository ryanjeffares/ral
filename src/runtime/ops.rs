#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Op {
    AssignLocal,
    AssignMember,
    CallComponent,
    DeclareLocal,
    LoadArg,
    LoadConstant,
    LoadLocal,
    LoadMember,
    Output,
    Print,
    PrintEmpty,
    PrintLn,
    PrintLnEmpty,
}
