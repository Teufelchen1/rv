use std::array;

use crate::instructions::Instruction;

use crate::decoder::decode;

use crate::executer::exec;

use crate::memory::Memory;

use crate::periph::MmapPeripheral;

use crate::register::Register;

use elf::abi;
use elf::endian::AnyEndian;
use elf::ElfBytes;

const LOG_LENGTH: usize = 20;

pub struct CPU<'trait_periph> {
    pub register: Register,
    pub memory: Memory<'trait_periph>,
    instruction_log: [Option<(usize, Instruction)>; LOG_LENGTH],
}

impl<'trait_periph> CPU<'trait_periph> {
    pub fn default(file: &[u8], uart: &'trait_periph mut dyn MmapPeripheral) -> Self {
        let mut cpu = Self {
            register: Register::default(),
            memory: Memory::default_hifive(uart),
            instruction_log: array::from_fn(|_| None),
        };

        let elffile =
            ElfBytes::<AnyEndian>::minimal_parse(file).expect("Failed to parse provided ELF file");

        if let Some(segments) = elffile.segments() {
            for phdr in segments {
                if phdr.p_type == abi::PT_LOAD {
                    if let Ok(mut addr) = usize::try_from(phdr.p_paddr) {
                        if cpu.memory.is_rom(addr) {
                            for i in elffile.segment_data(&phdr).unwrap() {
                                cpu.memory.rom[addr - cpu.memory.rom_base] = *i;
                                addr += 1;
                            }
                        } else if cpu.memory.is_ram(addr) {
                            for i in elffile.segment_data(&phdr).unwrap() {
                                cpu.memory.ram[addr - cpu.memory.ram_base] = *i;
                                addr += 1;
                            }
                        }
                    } else {
                        panic!("Could not get PT_LOAD address in your ELF file.");
                    }
                }
            }
        } else {
            panic!("Could not find segments in your ELF file.");
        }

        cpu.register.pc =
            u32::try_from(elffile.ehdr.e_entry).expect("Failed to read start address e_entry");

        cpu
    }

    pub fn instruction_at_addr(&self, addr: usize) -> Result<Instruction, &'static str> {
        decode(self.memory.read_word(addr))
    }

    pub fn current_instruction(&self) -> Result<(usize, Instruction), &'static str> {
        let addr = self.register.pc as usize;
        let inst = self.instruction_at_addr(addr)?;
        Ok((addr, inst))
    }

    #[allow(dead_code)]
    pub fn next_instruction(&self) -> Result<(usize, Instruction), &'static str> {
        let (cur_addr, cur_inst) = self.current_instruction()?;
        let addr = {
            if cur_inst.is_compressed() {
                cur_addr + 2
            } else {
                cur_addr + 4
            }
        };
        let inst = self.instruction_at_addr(addr)?;
        Ok((addr, inst))
    }

    pub fn next_n_instructions(&self, n: usize) -> Vec<(usize, Result<Instruction, u32>)> {
        let mut instruction_list = Vec::new();
        let mut addr = self.register.pc as usize;
        for _ in 0..n {
            let cur_inst = self.instruction_at_addr(addr);
            if let Ok(inst) = cur_inst {
                let compressed = inst.is_compressed();
                instruction_list.push((addr, Ok(inst)));
                if compressed {
                    addr += 2;
                } else {
                    addr += 4;
                }
            } else {
                instruction_list.push((addr, Err(self.memory.read_word(addr))));
                addr += 4;
            }
        }
        instruction_list
    }

    pub fn _last_instruction(&self) -> &Option<(usize, Instruction)> {
        self.instruction_log.last().unwrap_or(&None)
    }

    pub fn last_n_instructions(&self, n: usize) -> &[Option<(usize, Instruction)>] {
        if n > self.instruction_log.len() {
            &self.instruction_log
        } else {
            &self.instruction_log[self.instruction_log.len() - n..]
        }
    }

    /// Returns true for all instructions except when executing ebreak.
    /// ebreak is used to signaling the termination of the programm.
    pub fn step(&mut self) -> Result<bool, &'static str> {
        let (addr, inst) = self.current_instruction()?;
        exec(&mut self.register, &mut self.memory, &inst, true, true);
        self.instruction_log.rotate_left(1);
        self.instruction_log[LOG_LENGTH - 1] = Some((addr, inst.clone()));
        Ok(!matches!(inst, Instruction::EBREAK()))
    }
}
