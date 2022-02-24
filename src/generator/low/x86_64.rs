use crate::generator::high::{self, CompilationUnit, Function, USize64};
use iced_x86 as iced;
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
    let mut ass = CodeAssembler::new(64)?;

    let mut labels = HashMap::new();
    type RegisterFreq = HashMap<high::Register, usize>;
    let mut registers: RegisterFreq = HashMap::new();

    // Helper functions for counting registers in use
    fn add_reg(reg: high::Register, registers: &mut RegisterFreq) {
        *registers.entry(reg).or_default() += 1;
    }
    fn add_reg_storage(val: high::LValue<USize64>, registers: &mut RegisterFreq) {
        if let high::LValue::Reg(reg) = val {
            add_reg(reg, registers);
        }
    }
    fn add_reg_rvalue(val: high::RValue<USize64>, registers: &mut RegisterFreq) {
        if let high::RValue::Writeable(val) = val {
            add_reg_storage(val, registers);
        }
    }
    fn add_binary_op(p: &high::BinaryOperator<USize64>, registers: &mut RegisterFreq) {
        add_reg_rvalue(p.a, registers);
        add_reg_rvalue(p.b, registers);
        add_reg_storage(p.dst, registers);
    }

    // First we need to fill in mapping between jump destinations and the label that iced will use
    // to jump there. We need this because we can only create labels in place, so we need to know
    // beforehand which parts we are going to jump to
    //
    // In this pass we will also identify which virtual registers are used so we can allocate
    // physical registers using `registers`
    use high::Instruction::*;
    use high::{LValue, PrimitiveValue, RValue};
    let ins = &func.instructions;
    for (i, ins) in ins.iter().enumerate() {
        match ins {
            Move { src, dst, value: _ } => {
                add_reg_rvalue(*src, &mut registers);
                add_reg_storage(*dst, &mut registers);
            }
            LoadParameter { n, dst } => {}
            Add(p) => add_binary_op(p, &mut registers),
            Subtract(p) => add_binary_op(p, &mut registers),
            Multiply(p) => add_binary_op(p, &mut registers),
            Divide(p) => add_binary_op(p, &mut registers),
            Call {
                function: _,
                return_value,
            } => {
                if let Some(return_value) = return_value {
                    add_reg_storage(*return_value, &mut registers);
                }
            }
            Return { value } => {
                add_reg_rvalue(*value, &mut registers);
            }
            Jump { offset: _ } => {}
            ConditionalJump {
                offset,
                value,
                condition: _,
            } => {
                add_reg_storage(*value, &mut registers);
                let dst = func.compute_ins_offset(i, *offset).unwrap();
                labels.entry(dst).or_insert_with(|| ass.create_label());
            }
        }
    }
    // Currently we can use rax, r10, r11
    // TODO: improve this to use registers that don't hold parameters / do analysis to re-use
    // registers that line up when the `LoadParameter` instruction is used. Also compute spill off
    // based on the access count (the value in `registers`)
    const VOLATILE_PHYSICAL_REGISTER_COUNT: usize = 3;
    if registers.len() > VOLATILE_PHYSICAL_REGISTER_COUNT {
        unimplemented!("Too many registers used! {:?}", registers);
    }

    let available_phys_regs64 = [rax, r10, r11];
    let available_phys_regs32 = [eax, r10d, r11d];
    // Mapping between virtual and physical registers
    let phys_regs64: HashMap<high::Register, AsmRegister64> = registers
        .keys()
        .enumerate()
        .map(|(i, reg)| (*reg, available_phys_regs64[i]))
        .collect();

    let phys_regs32: HashMap<high::Register, AsmRegister32> = registers
        .keys()
        .enumerate()
        .map(|(i, reg)| (*reg, available_phys_regs32[i]))
        .collect();

    let map_register64 = |reg: high::Register| -> iced::code_asm::AsmRegister64 { phys_regs64[&reg] };
    let map_register32 = |reg: high::Register| -> iced::code_asm::AsmRegister32 { phys_regs32[&reg] };

    for (i, ins) in ins.iter().enumerate() {
        // A jump in this function wants to jump to this location, set the label's location for iced
        if let Some(label) = labels.get_mut(&i) {
            ass.set_label(label)?;
        }
        println!("Processing {:?}", ins);
        match ins {
            Move { src, dst, value } => {
                match (*dst, *src) {
                    (LValue::Reg(dst), RValue::Writeable(LValue::Reg(src))) => {
                        ass.mov(map_register64(dst), map_register64(src))?
                    }
                    // TODO: Respect integer sizes.
                    // There is no add 64 bit register with 64 bit constant so wed have to use a
                    // temp one
                    (LValue::Reg(dst), RValue::Literal(src)) => {
                        ass.mov(map_register64(dst), src as i64)?
                    }
                    rest => unimplemented!("({:?})", rest),
                }
            }
            LoadParameter { n, dst } => {}
            Add(p) => {
                let operands = p.to_two_args().expect("Bad ir"); // FIXME
                match operands {
                    (LValue::Reg(a), RValue::Writeable(LValue::Reg(b))) => {
                        ass.add(map_register64(a), map_register64(b))?
                    }
                    // TODO: Respect integer sizes.
                    // There is no add 64 bit register with 64 bit constant so wed have to use a
                    // temp one
                    (LValue::Reg(a), RValue::Literal(lit)) => {
                        ass.add(map_register64(a), lit as i32)?
                    }
                    rest => unimplemented!("({:?})", rest),
                }
            }
            Subtract(p) => {
                let operands = p.to_two_args().expect("Bad ir"); // FIXME
                match operands {
                    (LValue::Reg(a), RValue::Writeable(LValue::Reg(b))) => {
                        ass.sub(map_register64(a), map_register64(b))?
                    }
                    (LValue::Reg(a), RValue::Literal(lit)) => {
                        ass.sub(map_register64(a), lit as i32)?
                    }
                    rest => unimplemented!("({:?})", rest),
                }
            }
            Multiply(p) => {
                let operands = p.to_two_args().expect("Bad ir"); // FIXME

                match operands {
                    (LValue::Reg(a), RValue::Writeable(LValue::Reg(b))) => {
                        match p.value {
                            PrimitiveValue::Signed(bits) => {
                                ass.imul_2(map_register64(a), map_register64(b))?
                            }
                            //TODO: use unsigned multiply
                            PrimitiveValue::Unsigned(bits) => {
                                ass.imul_2(map_register64(a), map_register64(b))?
                            }
                            _ => unimplemented!("No floating point"),
                        }
                    }
                    (LValue::Reg(a), RValue::Literal(lit)) => {
                        println!("Mul {:?} with {:?}", a, lit);
                        let mut skip_data = ass.create_label();
                        ass.jmp(skip_data)?;
                        let data = ass.create_label();
                        ass.db(&(lit as u32).to_ne_bytes())?;
                        ass.set_label(&mut skip_data)?;

                        //Inner block is the same
                        match p.value {
                            PrimitiveValue::Signed(bits) => {
                                match bits {
                                    high::IntegerSize::B32 => {}
                                    _ => unimplemented!("Only 32 bit mutiply is supported"),
                                }
                                println!("mul");
                                ass.imul_2(map_register32(a), ptr(data))?
                            }
                            //TODO: use unsigned multiply
                            PrimitiveValue::Unsigned(bits) => {
                                match bits {
                                    high::IntegerSize::B32 => {}
                                    _ => unimplemented!("Only 32 bit mutiply is supported"),
                                }
                                println!("mul");
                                ass.imul_2(map_register32(a), ptr(data))?
                            }
                            _ => unimplemented!("No floating point"),
                        }
                    }
                    rest => unimplemented!("({:?})", rest),
                }
            }
            Divide(p) => {
                let operands = p.to_two_args().expect("Bad ir"); // FIXME
                match operands {
                    (LValue::Reg(a), RValue::Writeable(LValue::Reg(b))) => {
                        //ass.div(map_register64(a), map_register64(b))?
                    }
                    (LValue::Reg(a), RValue::Literal(lit)) => {
                        //ass.div(map_register64(a), lit as i32)?
                    }
                    rest => unimplemented!("({:?})", rest),
                }
            }
            Call {
                function,
                return_value,
            } => {}
            Return { value } => {}
            Jump { offset } => {}
            ConditionalJump {
                offset,
                value,
                condition,
            } => {}
        }
    }

    Ok(ass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        use high::{
            self, BinaryOperator as B, Instruction, IntegerSize, LValue, PrimitiveValue, RValue,
        };
        let one = RValue::Literal::<USize64>(1);
        let two = RValue::Literal::<USize64>(2);
        let five = RValue::Literal::<USize64>(5);

        let mut alloc = high::RegisterAllocator::new();
        let r1 = LValue::Reg(alloc.alloc());
        let r2 = LValue::Reg(alloc.alloc());
        let program = vec![
            // r1 = 1
            Instruction::Move {
                src: one,
                dst: r1,
                value: PrimitiveValue::Signed(IntegerSize::B32),
            },
            // r1 = 2 + r1   (==3)
            Instruction::Add(B {
                a: two,
                b: RValue::Writeable(r1),
                dst: r1,
                value: PrimitiveValue::Signed(IntegerSize::B32),
            }),
            // r1 = r2 * 2   (==6)
            Instruction::Multiply(B {
                a: RValue::Writeable(r1),
                b: two,
                dst: r1,
                value: PrimitiveValue::Signed(IntegerSize::B32),
            }),
            // r2 = 1
            Instruction::Move {
                src: two,
                dst: r2,
                value: PrimitiveValue::Signed(IntegerSize::B32),
            },
            // r2 = r1 / r2
            Instruction::Divide(B {
                a: RValue::Writeable(r1),
                b: RValue::Writeable(r2),
                dst: r2,
                value: PrimitiveValue::Signed(IntegerSize::B32),
            }),
            // r2 = r2 * 5
            Instruction::Multiply(B {
                a: RValue::Writeable(r2),
                b: five,
                dst: r2,
                value: PrimitiveValue::Signed(IntegerSize::B32),
            }),
            // Jump to top
            /*Instruction::Jump { offset: -5 },
            // Return r2
            Instruction::Return {
                value: RValue::Writeable(r2),
            },*/
        ];

        let function = Function::new("test", program);
        let mut assembler = gen_function(function).unwrap();
        let ip = 0;
        let bytes = assembler.assemble(0).unwrap();
        print_disassembly(&bytes, ip)
    }

    fn print_disassembly(bytes: &[u8], ip: u64) {
        use iced_x86::{Decoder, DecoderOptions, Formatter, NasmFormatter};
        let mut decoder = Decoder::with_ip(64, &bytes, ip, DecoderOptions::NONE);

        // Formatters: Masm*, Nasm*, Gas* (AT&T) and Intel* (XED).
        // For fastest code, see `SpecializedFormatter` which is ~3.3x faster. Use it if formatting
        // speed is more important than being able to re-assemble formatted instructions.
        let mut formatter = NasmFormatter::new();

        // Change some options, there are many more
        formatter.options_mut().set_digit_separator("`");
        formatter.options_mut().set_first_operand_char_index(10);

        // String implements FormatterOutput
        let mut output = String::new();

        // Initialize this outside the loop because decode_out() writes to every field
        let mut instruction = iced::Instruction::default();

        // The decoder also implements Iterator/IntoIterator so you could use a for loop:
        //      for instruction in &mut decoder { /* ... */ }
        // or collect():
        //      let instructions: Vec<_> = decoder.into_iter().collect();
        // but can_decode()/decode_out() is a little faster:
        while decoder.can_decode() {
            // There's also a decode() method that returns an instruction but that also
            // means it copies an instruction (40 bytes):
            //     instruction = decoder.decode();
            decoder.decode_out(&mut instruction);
            let mut jmp_to = None;
            if instruction.is_jmp_short() {
                let target = instruction.near_branch64();
                println!("{:?}", target);
                jmp_to = Some(target);
                //instruction.as_short_branch
            }

            // Format the instruction ("disassemble" it)
            output.clear();
            formatter.format(&instruction, &mut output);

            // Eg. "00007FFAC46ACDB2 488DAC2400FFFFFF     lea       rbp,[rsp-100h]"
            print!("{:016X} ", instruction.ip());
            let start_index = (instruction.ip() - ip) as usize;
            let instr_bytes = &bytes[start_index..start_index + instruction.len()];
            for b in instr_bytes.iter() {
                print!("{:02X}", b);
            }
            let col_width = 24;
            if instr_bytes.len() < col_width {
                for _ in 0..col_width - instr_bytes.len() {
                    print!("  ");
                }
            }
            println!(" {}", output);
            if let Some(ip) = jmp_to {
                //println!("Setting ip to {}", ip);
                //decoder.set_ip(ip);
            }
        }
    }
}
