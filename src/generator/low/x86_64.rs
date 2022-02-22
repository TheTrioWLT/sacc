use std::collections::HashMap;

use crate::generator::high::{self, CompilationUnit, USize64, Function};

use iced_x86::code_asm::*;

pub fn do_codegen(unit: CompilationUnit<'_, '_, USize64>) -> Result<(), IcedError> {
    //Build list of indices that are jumped to because `unit` only has jump instructions with the
    //destination
    let mut a = CodeAssembler::new(64)?;

    // Anytime you add something to a register (or subtract from it), you create a
    // memory operand. You can also call word_ptr(), dword_bcst() etc to create memory
    // operands.
    let _ = rax; // register
    let _ = rax + 0; // memory with no size hint
    let _ = ptr(rax); // memory with no size hint
    let _ = rax + rcx * 4 - 123; // memory with no size hint
                                 // To create a memory operand with only a displacement or only a base register,
                                 // you can call one of the memory fns:
    let _ = qword_ptr(123); // memory with a qword size hint
    let _ = dword_bcst(rcx); // memory (broadcast) with a dword size hint
                             // To add a segment override, call the segment methods:
    let _ = ptr(rax).fs(); // fs:[rax]

    // Each mnemonic is a method
    a.push(rcx)?;
    // There are a few exceptions where you must append `_<opcount>` to the mnemonic to
    // get the instruction you need:
    a.ret()?;
    a.ret_1(123)?;
    // Use byte_ptr(), word_bcst(), etc to force the arg to a memory operand and to add a
    // size hint
    a.xor(byte_ptr(rdx + r14 * 4 + 123), 0x10)?;
    // Prefixes are also methods
    a.rep().stosd()?;
    // Sometimes, you must add an integer suffix to help the compiler:
    a.mov(rax, 0x1234_5678_9ABC_DEF0u64)?;

    // Create labels that can be referenced by code
    let mut loop_lbl1 = a.create_label();
    let mut after_loop1 = a.create_label();
    a.mov(ecx, 10)?;
    a.set_label(&mut loop_lbl1)?;
    a.dec(ecx)?;
    a.jp(after_loop1)?;
    a.jne(loop_lbl1)?;
    a.set_label(&mut after_loop1)?;

    // It's possible to reference labels with RIP-relative addressing
    let mut skip_data = a.create_label();
    let mut data = a.create_label();
    a.jmp(skip_data)?;
    a.set_label(&mut data)?;
    a.db(b"\x90\xCC\xF1\x90")?;
    a.set_label(&mut skip_data)?;
    a.lea(rax, ptr(data))?;

    // Encode all added instructions
    let ip = 0;
    let bytes = a.assemble(ip)?;

    Ok(())
}

fn gen_function(func: Function<'_, USize64>) -> Result<CodeAssembler, IcedError> {
    let mut a = CodeAssembler::new(64)?;
    let mut labels = HashMap::new();
    let ins = &func.instructions;
    for i in 0..ins.len() {
        if let high::Instruction::ConditionalJump { offset, value, condition } = ins[i].clone() {
            let dst: usize = (offset + i as isize).try_into()?;
        }
    }

    Ok(a)
}
