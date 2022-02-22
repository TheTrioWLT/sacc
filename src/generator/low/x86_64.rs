use crate::generator::high::{self, CompilationUnit, Function, USize64};
use iced_x86::code_asm::*;
use std::collections::HashMap;

pub fn do_codegen(unit: CompilationUnit<'_, USize64>) -> Result<(), IcedError> {
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

    //First we need to fill in mapping between jump destinations and the label that iced will use
    //to jump there. We need this because we can only create labels in place, so we need to know
    //beforehand which parts we are going to jump to
    let mut labels = HashMap::new();
    let ins = &func.instructions;
    for (i,_) in ins.iter().enumerate() {
        if let high::Instruction::ConditionalJump {
            offset,
            value: _,
            condition: _,
        } = &ins[i]
        {
            let dst = func.compute_ins_offset(i, *offset).unwrap();
            labels.entry(dst).or_insert_with(|| a.create_label());
        }
    }
    for (i, ins) in ins.iter().enumerate() {
        // A jump in this function wants to jump to this location, set the label's location for iced
        if let Some(label) = labels.get_mut(&i) {
            a.set_label(label)?;
        }
        use high::Instruction::*;
        match ins {
            Move {src, dst, value } => {}
            Add {a, b, dst, value } => {}
            Subtract {a, b, dst, value } => {}
            Multiply {a, b, dst, value } => {}
            Divide {a, b, dst, value } => {}
            Call { function, return_value } => {}
            Return { value } => {}
            Jump { offset } => {}
            ConditionalJump { offset, value, condition } => {}
            _ => {}
        }
    }

    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        use high::{
            Instruction, IntegerSize, PrimitiveValue, RValue, RegisterAllocator, StorageLocation,
        };
        let one = RValue::Literal::<USize64>(1);
        let two = RValue::Literal::<USize64>(2);
        let five = RValue::Literal::<USize64>(5);
        let mut alloc = RegisterAllocator::new();
        let r1 = StorageLocation::Reg(alloc.alloc());
        let r2 = StorageLocation::Reg(alloc.alloc());
        let program = vec![
            // r1 = 1
            Instruction::Move {
                src: one,
                dst: r1,
                value: PrimitiveValue::Signed(IntegerSize::B64),
            },
            // r1 = 2 + r1   (==3)
            Instruction::Add {
                a: two,
                b: RValue::Writeable(r1),
                dst: r1,
                value: PrimitiveValue::Signed(IntegerSize::B64),
            },
            // r1 = r2 * 2   (==6)
            Instruction::Multiply {
                a: RValue::Writeable(r1),
                b: two,
                dst: r1,
                value: PrimitiveValue::Signed(IntegerSize::B64),
            },
            // r2 = 1
            Instruction::Move {
                src: two,
                dst: r2,
                value: PrimitiveValue::Signed(IntegerSize::B64),
            },
            // r2 = r1 / r2
            Instruction::Divide {
                a: RValue::Writeable(r1),
                b: RValue::Writeable(r2),
                dst: r2,
                value: PrimitiveValue::Signed(IntegerSize::B64),
            },
            // r1 = r2 * 5
            Instruction::Divide {
                a: RValue::Writeable(r2),
                b: five,
                dst: r1,
                value: PrimitiveValue::Signed(IntegerSize::B64),
            },
            // Jump to top
            Instruction::Jump { offset: -5 },
            // Return r1
            Instruction::Return { value: RValue::Writeable(r1) }
        ];

        let function = Function::new("test", program);
        let mut assembler = gen_function(function).unwrap();
        let bytes = assembler.assemble(0).unwrap();
        println!("Bytes: {:?}", bytes);
    }
}
