//! IR to eBPF compiler
//!
//! Compiles Nushell's IR (IrBlock) to eBPF bytecode.

use std::collections::HashMap;

use nu_protocol::ast::{Math, Bits, Comparison, Operator};
use nu_protocol::engine::EngineState;
use nu_protocol::ir::{Instruction, IrBlock, Literal};
use nu_protocol::{DeclId, RegId, VarId};

use super::elf::{BpfFieldType, BpfMapDef, EbpfMap, EventSchema, MapRelocation, SchemaField};
use super::instruction::{BpfHelper, EbpfBuilder, EbpfInsn, EbpfReg, opcode};
use super::CompileError;

/// Result of compiling IR to eBPF
pub struct CompileResult {
    /// The compiled bytecode
    pub bytecode: Vec<u8>,
    /// Maps needed by the program
    pub maps: Vec<EbpfMap>,
    /// Relocations for map references
    pub relocations: Vec<MapRelocation>,
    /// Optional schema for structured events
    pub event_schema: Option<EventSchema>,
}

/// Name of the perf event array map for output
const PERF_MAP_NAME: &str = "events";

/// Name of the counter hash map for bpf-count
const COUNTER_MAP_NAME: &str = "counters";

/// Architecture-specific pt_regs offsets for function arguments
///
/// These are the byte offsets into struct pt_regs where each function
/// argument register is stored.
#[cfg(target_arch = "x86_64")]
mod pt_regs_offsets {
    /// Offsets for arguments 0-5 (rdi, rsi, rdx, rcx, r8, r9)
    pub const ARG_OFFSETS: [i16; 6] = [
        112, // arg0: rdi
        104, // arg1: rsi
        96,  // arg2: rdx
        88,  // arg3: rcx
        72,  // arg4: r8
        64,  // arg5: r9
    ];
    /// Offset for return value (rax)
    pub const RETVAL_OFFSET: i16 = 80;
}

#[cfg(target_arch = "aarch64")]
mod pt_regs_offsets {
    /// Offsets for arguments 0-7 (x0-x7, each 8 bytes)
    pub const ARG_OFFSETS: [i16; 8] = [
        0,  // arg0: x0
        8,  // arg1: x1
        16, // arg2: x2
        24, // arg3: x3
        32, // arg4: x4
        40, // arg5: x5
        48, // arg6: x6
        56, // arg7: x7
    ];
    /// Offset for return value (x0)
    pub const RETVAL_OFFSET: i16 = 0;
}

// Fallback for unsupported architectures (compilation will fail at runtime)
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
mod pt_regs_offsets {
    pub const ARG_OFFSETS: [i16; 6] = [0; 6];
    pub const RETVAL_OFFSET: i16 = 0;
}

/// Maps Nushell register IDs to eBPF registers or stack locations
pub struct RegisterAllocator {
    /// Maps Nu RegId -> eBPF register
    reg_mapping: HashMap<u32, EbpfReg>,
    /// Maps Nu VarId -> eBPF register (for variables)
    var_mapping: HashMap<usize, EbpfReg>,
    /// Next available callee-saved register (r6-r9)
    next_saved: u8,
}

impl RegisterAllocator {
    pub fn new() -> Self {
        Self {
            reg_mapping: HashMap::new(),
            var_mapping: HashMap::new(),
            next_saved: 6, // Start at r6 (callee-saved)
        }
    }

    /// Allocate a new eBPF register
    /// Note: R9 is reserved for saving the context pointer (R1)
    fn alloc_register(&mut self) -> Result<EbpfReg, CompileError> {
        if self.next_saved <= 8 {
            let ebpf_reg = match self.next_saved {
                6 => EbpfReg::R6,
                7 => EbpfReg::R7,
                8 => EbpfReg::R8,
                _ => unreachable!(),
            };
            self.next_saved += 1;
            Ok(ebpf_reg)
        } else {
            Err(CompileError::RegisterExhaustion)
        }
    }

    /// Get or allocate an eBPF register for the given Nushell register
    pub fn get_or_alloc(&mut self, reg: RegId) -> Result<EbpfReg, CompileError> {
        let reg_id = reg.get();

        if let Some(&ebpf_reg) = self.reg_mapping.get(&reg_id) {
            return Ok(ebpf_reg);
        }

        let ebpf_reg = self.alloc_register()?;
        self.reg_mapping.insert(reg_id, ebpf_reg);
        Ok(ebpf_reg)
    }

    /// Get the eBPF register for a Nushell register (must already be allocated)
    pub fn get(&self, reg: RegId) -> Result<EbpfReg, CompileError> {
        self.reg_mapping
            .get(&reg.get())
            .copied()
            .ok_or_else(|| CompileError::UnsupportedInstruction(
                format!("Register %{} not allocated", reg.get())
            ))
    }

    /// Get or allocate an eBPF register for a Nushell variable
    pub fn get_or_alloc_var(&mut self, var_id: VarId) -> Result<EbpfReg, CompileError> {
        let var_num = var_id.get();

        if let Some(&ebpf_reg) = self.var_mapping.get(&var_num) {
            return Ok(ebpf_reg);
        }

        let ebpf_reg = self.alloc_register()?;
        self.var_mapping.insert(var_num, ebpf_reg);
        Ok(ebpf_reg)
    }

    /// Get the eBPF register for a Nushell variable (must already be allocated)
    pub fn get_var(&self, var_id: VarId) -> Result<EbpfReg, CompileError> {
        self.var_mapping
            .get(&var_id.get())
            .copied()
            .ok_or_else(|| CompileError::UnsupportedInstruction(
                format!("Variable ${} not allocated", var_id.get())
            ))
    }
}

/// Pending jump that needs to be fixed up
struct PendingJump {
    /// Index in builder where the jump instruction is
    ebpf_insn_idx: usize,
    /// Target IR instruction index
    target_ir_idx: usize,
}

/// Tracks a field being built in a record
#[derive(Debug, Clone)]
struct RecordFieldBuilder {
    /// Field name
    name: String,
    /// Stack offset where the field value is stored (relative to R10)
    stack_offset: i16,
    /// Type of the field (determined from how the value was computed)
    field_type: BpfFieldType,
}

/// Tracks a record being built
#[derive(Debug, Clone, Default)]
struct RecordBuilder {
    /// Fields in the order they were inserted
    fields: Vec<RecordFieldBuilder>,
    /// Current write offset within the record (relative to record start)
    current_offset: i16,
    /// Base stack offset for this record (relative to R10)
    base_offset: i16,
}

/// Compiles Nushell IR to eBPF bytecode
pub struct IrToEbpfCompiler<'a> {
    ir_block: &'a IrBlock,
    engine_state: &'a EngineState,
    builder: EbpfBuilder,
    reg_alloc: RegisterAllocator,
    /// Maps IR instruction index -> eBPF instruction index
    ir_to_ebpf: HashMap<usize, usize>,
    /// Pending jumps to fix up
    pending_jumps: Vec<PendingJump>,
    /// Whether the program needs a perf event map for output
    needs_perf_map: bool,
    /// Whether the program needs a counter hash map
    needs_counter_map: bool,
    /// Relocations for map references
    relocations: Vec<MapRelocation>,
    /// Current stack offset for temporary storage (grows negative from R10)
    stack_offset: i16,
    /// We need to save R1 (context) at the start if we use bpf-emit
    ctx_saved: bool,
    /// Pushed positional arguments for the next call (register IDs)
    pushed_args: Vec<RegId>,
    /// Track literal integer values loaded into registers (for compile-time constants)
    literal_values: HashMap<u32, i64>,
    /// Track literal string values loaded into registers (for field names)
    literal_strings: HashMap<u32, String>,
    /// Track records being built (RegId -> RecordBuilder)
    record_builders: HashMap<u32, RecordBuilder>,
    /// Track the type of value produced by each register (for schema inference)
    register_types: HashMap<u32, BpfFieldType>,
    /// The event schema if structured events are used
    event_schema: Option<EventSchema>,
}

impl<'a> IrToEbpfCompiler<'a> {
    /// Compile an IrBlock to eBPF bytecode (simple version, ignores maps)
    pub fn compile(ir_block: &'a IrBlock, engine_state: &'a EngineState) -> Result<Vec<u8>, CompileError> {
        let result = Self::compile_full(ir_block, engine_state)?;
        Ok(result.bytecode)
    }

    /// Compile an IrBlock to eBPF bytecode with full result including maps
    pub fn compile_full(ir_block: &'a IrBlock, engine_state: &'a EngineState) -> Result<CompileResult, CompileError> {
        Self::compile_inner(ir_block, Some(engine_state))
    }

    /// Compile without engine state (for tests, will fail on Call instructions)
    #[cfg(test)]
    pub fn compile_no_calls(ir_block: &'a IrBlock) -> Result<Vec<u8>, CompileError> {
        let result = Self::compile_inner(ir_block, None)?;
        Ok(result.bytecode)
    }

    fn compile_inner(ir_block: &'a IrBlock, engine_state: Option<&'a EngineState>) -> Result<CompileResult, CompileError> {
        // Create a dummy engine state for when we don't have one
        // This will only be accessed if there's a Call instruction
        static DUMMY: std::sync::OnceLock<EngineState> = std::sync::OnceLock::new();
        let dummy_state = DUMMY.get_or_init(EngineState::new);
        let engine_state = engine_state.unwrap_or(dummy_state);

        let mut compiler = IrToEbpfCompiler {
            ir_block,
            engine_state,
            builder: EbpfBuilder::new(),
            reg_alloc: RegisterAllocator::new(),
            ir_to_ebpf: HashMap::new(),
            pending_jumps: Vec::new(),
            needs_perf_map: false,
            needs_counter_map: false,
            relocations: Vec::new(),
            stack_offset: -8, // Start at -8 from R10
            ctx_saved: false,
            pushed_args: Vec::new(),
            literal_values: HashMap::new(),
            literal_strings: HashMap::new(),
            record_builders: HashMap::new(),
            register_types: HashMap::new(),
            event_schema: None,
        };

        // Save the context pointer (R1) to R9 at the start
        // This is needed for bpf_perf_event_output which requires the context
        // R1 gets clobbered by helper calls, so we save it in a callee-saved register
        compiler.builder.push(EbpfInsn::mov64_reg(EbpfReg::R9, EbpfReg::R1));
        compiler.ctx_saved = true;

        // Compile each instruction, tracking IR->eBPF index mapping
        for (idx, instr) in ir_block.instructions.iter().enumerate() {
            // Record the eBPF instruction index before compiling this IR instruction
            compiler.ir_to_ebpf.insert(idx, compiler.builder.len());
            compiler.compile_instruction(instr, idx)?;
        }
        // Record end position for jumps targeting past the last instruction
        compiler.ir_to_ebpf.insert(ir_block.instructions.len(), compiler.builder.len());

        // Fix up pending jumps
        compiler.fixup_jumps()?;

        // Ensure we have an exit instruction
        if compiler.builder.is_empty() {
            // Empty program - just return 0
            compiler.builder.push(EbpfInsn::mov64_imm(EbpfReg::R0, 0));
            compiler.builder.push(EbpfInsn::exit());
        }

        // Build the result
        let mut maps = Vec::new();
        if compiler.needs_perf_map {
            maps.push(EbpfMap {
                name: PERF_MAP_NAME.to_string(),
                def: BpfMapDef::perf_event_array(),
            });
        }
        if compiler.needs_counter_map {
            maps.push(EbpfMap {
                name: COUNTER_MAP_NAME.to_string(),
                def: BpfMapDef::counter_hash(),
            });
        }

        Ok(CompileResult {
            bytecode: compiler.builder.build(),
            maps,
            relocations: compiler.relocations,
            event_schema: compiler.event_schema,
        })
    }

    /// Fix up pending jump instructions with correct offsets
    fn fixup_jumps(&mut self) -> Result<(), CompileError> {
        for jump in &self.pending_jumps {
            let target_ebpf_idx = self.ir_to_ebpf.get(&jump.target_ir_idx)
                .ok_or_else(|| CompileError::UnsupportedInstruction(
                    format!("Invalid jump target: IR instruction {}", jump.target_ir_idx)
                ))?;

            // eBPF jump offset is relative to the NEXT instruction
            // offset = target - (current + 1)
            let offset = (*target_ebpf_idx as i32) - (jump.ebpf_insn_idx as i32) - 1;

            if offset < i16::MIN as i32 || offset > i16::MAX as i32 {
                return Err(CompileError::UnsupportedInstruction(
                    format!("Jump offset {} out of range", offset)
                ));
            }

            self.builder.set_offset(jump.ebpf_insn_idx, offset as i16);
        }
        Ok(())
    }

    fn compile_instruction(&mut self, instr: &Instruction, _idx: usize) -> Result<(), CompileError> {
        match instr {
            Instruction::LoadLiteral { dst, lit } => {
                self.compile_load_literal(*dst, lit)
            }
            Instruction::Move { dst, src } => {
                self.compile_move(*dst, *src)
            }
            Instruction::Clone { dst, src } => {
                // Clone is same as Move for our purposes (we don't track lifetimes)
                self.compile_move(*dst, *src)
            }
            Instruction::BinaryOp { lhs_dst, op, rhs } => {
                self.compile_binary_op(*lhs_dst, op, *rhs)
            }
            Instruction::Return { src } => {
                self.compile_return(*src)
            }
            Instruction::LoadVariable { dst, var_id } => {
                self.compile_load_variable(*dst, *var_id)
            }
            Instruction::StoreVariable { var_id, src } => {
                self.compile_store_variable(*var_id, *src)
            }
            Instruction::DropVariable { .. } => {
                // No-op in eBPF - we don't need to clean up
                Ok(())
            }
            Instruction::Not { src_dst } => {
                self.compile_not(*src_dst)
            }
            Instruction::BranchIf { cond, index } => {
                self.compile_branch_if(*cond, *index as usize)
            }
            Instruction::Jump { index } => {
                self.compile_jump(*index as usize)
            }
            Instruction::Call { decl_id, src_dst } => {
                self.compile_call(*decl_id, *src_dst)
            }
            // Instructions we can safely ignore for simple closures
            Instruction::Span { .. } => Ok(()),
            Instruction::PushPositional { src } => {
                // Track pushed argument for filter commands
                self.pushed_args.push(*src);
                Ok(())
            }
            Instruction::RedirectOut { .. } => Ok(()),
            Instruction::RedirectErr { .. } => Ok(()),
            Instruction::Drop { .. } => Ok(()),
            Instruction::Drain { .. } => Ok(()),
            Instruction::DrainIfEnd { .. } => Ok(()),
            Instruction::Collect { .. } => Ok(()),
            Instruction::RecordInsert { src_dst, key, val } => {
                self.compile_record_insert(*src_dst, *key, *val)
            }
            // Unsupported instructions
            other => Err(CompileError::UnsupportedInstruction(format!("{:?}", other))),
        }
    }

    fn compile_load_literal(&mut self, dst: RegId, lit: &Literal) -> Result<(), CompileError> {
        let ebpf_dst = self.reg_alloc.get_or_alloc(dst)?;

        match lit {
            Literal::Int(val) => {
                // Track the literal value for commands that need compile-time constants
                self.literal_values.insert(dst.get(), *val);

                // Check if value fits in i32 immediate
                if *val >= i32::MIN as i64 && *val <= i32::MAX as i64 {
                    self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, *val as i32));
                } else {
                    // For 64-bit values, we need LD_DW_IMM (two instruction slots)
                    self.emit_load_64bit_imm(ebpf_dst, *val);
                }
                Ok(())
            }
            Literal::Bool(b) => {
                self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, if *b { 1 } else { 0 }));
                Ok(())
            }
            Literal::Nothing => {
                // Nothing is represented as 0
                self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, 0));
                Ok(())
            }
            Literal::String(data_slice) => {
                // Get the string data from the IrBlock's data buffer
                let start = data_slice.start as usize;
                let end = start + data_slice.len as usize;
                let string_bytes = &self.ir_block.data[start..end];

                // Track the string value for field names in records
                if let Ok(s) = std::str::from_utf8(string_bytes) {
                    self.literal_strings.insert(dst.get(), s.to_string());
                }

                // Convert first 8 bytes of string to i64 for comparison
                // This matches how bpf-comm encodes process names
                let mut arr = [0u8; 8];
                let len = string_bytes.len().min(8);
                arr[..len].copy_from_slice(&string_bytes[..len]);
                let val = i64::from_le_bytes(arr);
                self.emit_load_64bit_imm(ebpf_dst, val);
                Ok(())
            }
            Literal::Record { .. } => {
                // Create a RecordBuilder for this register
                // Records are built on the stack - we'll allocate space as fields are added
                // For now, just track the starting position
                let record_builder = RecordBuilder {
                    fields: Vec::new(),
                    current_offset: 0,
                    base_offset: self.stack_offset, // Will be updated as fields are added
                };
                self.record_builders.insert(dst.get(), record_builder);
                // Records in eBPF are represented as 0 (a placeholder)
                self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, 0));
                Ok(())
            }
            _ => Err(CompileError::UnsupportedLiteral),
        }
    }

    fn compile_move(&mut self, dst: RegId, src: RegId) -> Result<(), CompileError> {
        let ebpf_src = self.reg_alloc.get(src)?;
        let ebpf_dst = self.reg_alloc.get_or_alloc(dst)?;

        if ebpf_src.as_u8() != ebpf_dst.as_u8() {
            self.builder.push(EbpfInsn::mov64_reg(ebpf_dst, ebpf_src));
        }
        Ok(())
    }

    fn compile_binary_op(&mut self, lhs_dst: RegId, op: &Operator, rhs: RegId) -> Result<(), CompileError> {
        let ebpf_lhs = self.reg_alloc.get(lhs_dst)?;
        let ebpf_rhs = self.reg_alloc.get(rhs)?;

        match op {
            // Math operations
            Operator::Math(math) => {
                match math {
                    Math::Add => {
                        self.builder.push(EbpfInsn::add64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Math::Subtract => {
                        self.builder.push(EbpfInsn::sub64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Math::Multiply => {
                        self.builder.push(EbpfInsn::mul64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Math::Divide | Math::FloorDivide => {
                        self.builder.push(EbpfInsn::div64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Math::Modulo => {
                        self.builder.push(EbpfInsn::mod64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    _ => return Err(CompileError::UnsupportedInstruction(
                        format!("Math operator {:?}", math)
                    )),
                }
            }
            // Bitwise operations
            Operator::Bits(bits) => {
                match bits {
                    Bits::BitOr => {
                        self.builder.push(EbpfInsn::or64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Bits::BitAnd => {
                        self.builder.push(EbpfInsn::and64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Bits::BitXor => {
                        self.builder.push(EbpfInsn::xor64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Bits::ShiftLeft => {
                        self.builder.push(EbpfInsn::lsh64_reg(ebpf_lhs, ebpf_rhs));
                    }
                    Bits::ShiftRight => {
                        self.builder.push(EbpfInsn::rsh64_reg(ebpf_lhs, ebpf_rhs));
                    }
                }
            }
            // Comparison operations - result is 0 or 1
            Operator::Comparison(cmp) => {
                self.compile_comparison(ebpf_lhs, cmp, ebpf_rhs)?;
            }
            _ => return Err(CompileError::UnsupportedInstruction(
                format!("Operator {:?}", op)
            )),
        }

        Ok(())
    }

    fn compile_comparison(&mut self, lhs: EbpfReg, cmp: &Comparison, rhs: EbpfReg) -> Result<(), CompileError> {
        // Comparison in eBPF is done via conditional jumps
        // We emit: if (lhs cmp rhs) goto +1; r0 = 0; goto +1; r0 = 1
        // But we need to put result back in lhs register

        // Strategy:
        // 1. mov lhs, 1 (assume true)
        // 2. if (comparison fails) goto skip
        // 3. mov lhs, 0
        // skip:

        // First, save lhs value to a temp if needed and set lhs = 0
        let temp = EbpfReg::R0; // Use R0 as temp
        self.builder.push(EbpfInsn::mov64_reg(temp, lhs));
        self.builder.push(EbpfInsn::mov64_imm(lhs, 0)); // Assume false

        // Emit conditional jump based on comparison type
        // If condition is TRUE, skip the next instruction (which would keep lhs=0)
        // and fall through to setting lhs=1
        let jump_opcode = match cmp {
            Comparison::Equal => opcode::BPF_JMP | opcode::BPF_JEQ | opcode::BPF_X,
            Comparison::NotEqual => opcode::BPF_JMP | opcode::BPF_JNE | opcode::BPF_X,
            Comparison::LessThan => opcode::BPF_JMP | opcode::BPF_JLT | opcode::BPF_X,
            Comparison::LessThanOrEqual => opcode::BPF_JMP | opcode::BPF_JLE | opcode::BPF_X,
            Comparison::GreaterThan => opcode::BPF_JMP | opcode::BPF_JGT | opcode::BPF_X,
            Comparison::GreaterThanOrEqual => opcode::BPF_JMP | opcode::BPF_JGE | opcode::BPF_X,
            _ => return Err(CompileError::UnsupportedInstruction(
                format!("Comparison {:?}", cmp)
            )),
        };

        // Jump over the "goto skip" if condition is true
        // temp (original lhs) cmp rhs -> if true, skip 1 instruction
        self.builder.push(EbpfInsn::new(
            jump_opcode,
            temp.as_u8(),
            rhs.as_u8(),
            1, // Skip 1 instruction
            0,
        ));

        // If we get here, condition was false, skip setting to 1
        self.builder.push(EbpfInsn::new(
            opcode::BPF_JMP | opcode::BPF_JA,
            0,
            0,
            1, // Skip 1 instruction
            0,
        ));

        // Set lhs = 1 (condition was true)
        self.builder.push(EbpfInsn::mov64_imm(lhs, 1));

        Ok(())
    }

    fn compile_return(&mut self, src: RegId) -> Result<(), CompileError> {
        let ebpf_src = self.reg_alloc.get(src)?;

        // Move result to R0 (return register) if not already there
        if ebpf_src.as_u8() != EbpfReg::R0.as_u8() {
            self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R0, ebpf_src));
        }

        self.builder.push(EbpfInsn::exit());
        Ok(())
    }

    fn compile_store_variable(&mut self, var_id: VarId, src: RegId) -> Result<(), CompileError> {
        let ebpf_src = self.reg_alloc.get(src)?;
        let ebpf_var = self.reg_alloc.get_or_alloc_var(var_id)?;

        // Copy the value to the variable's register
        if ebpf_src.as_u8() != ebpf_var.as_u8() {
            self.builder.push(EbpfInsn::mov64_reg(ebpf_var, ebpf_src));
        }
        Ok(())
    }

    fn compile_load_variable(&mut self, dst: RegId, var_id: VarId) -> Result<(), CompileError> {
        let ebpf_var = self.reg_alloc.get_var(var_id)?;
        let ebpf_dst = self.reg_alloc.get_or_alloc(dst)?;

        // Copy from variable's register to destination
        if ebpf_var.as_u8() != ebpf_dst.as_u8() {
            self.builder.push(EbpfInsn::mov64_reg(ebpf_dst, ebpf_var));
        }
        Ok(())
    }

    /// Emit a 64-bit immediate load (uses two instruction slots)
    fn emit_load_64bit_imm(&mut self, dst: EbpfReg, val: i64) {
        // LD_DW_IMM uses two 8-byte slots
        // First slot: opcode + lower 32 bits in imm
        // Second slot: upper 32 bits in imm
        let lower = val as i32;
        let upper = (val >> 32) as i32;

        self.builder.push(EbpfInsn::new(
            opcode::LD_DW_IMM,
            dst.as_u8(),
            0,
            0,
            lower,
        ));
        // Second instruction slot (pseudo-instruction)
        self.builder.push(EbpfInsn::new(
            0,
            0,
            0,
            0,
            upper,
        ));
    }

    /// Compile logical NOT (flip boolean: 0 -> 1, non-zero -> 0)
    fn compile_not(&mut self, src_dst: RegId) -> Result<(), CompileError> {
        let ebpf_reg = self.reg_alloc.get(src_dst)?;

        // In Nushell, NOT is logical (boolean), not bitwise
        // We want: if reg == 0 then 1 else 0
        // Strategy:
        // 1. jeq reg, 0, +2  (if reg == 0, skip to setting 1)
        // 2. mov reg, 0      (reg was non-zero, set to 0)
        // 3. ja +1           (skip setting to 1)
        // 4. mov reg, 1      (reg was 0, set to 1)
        self.builder.push(EbpfInsn::jeq_imm(ebpf_reg, 0, 2));
        self.builder.push(EbpfInsn::mov64_imm(ebpf_reg, 0));
        self.builder.push(EbpfInsn::jump(1));
        self.builder.push(EbpfInsn::mov64_imm(ebpf_reg, 1));

        Ok(())
    }

    /// Compile conditional branch (branch if cond is truthy)
    fn compile_branch_if(&mut self, cond: RegId, target_ir_idx: usize) -> Result<(), CompileError> {
        let ebpf_cond = self.reg_alloc.get(cond)?;

        // Branch if cond != 0
        // We'll use JNE with imm=0, but eBPF JNE with imm requires BPF_K
        // Actually we need to compare against 0 - if non-zero, jump
        // Use: jeq cond, 0, +1; ja target
        // If cond == 0, skip the jump. Otherwise, jump.
        // But we want to jump if truthy, so:
        // jne cond, 0, target (jump if cond != 0)

        // eBPF doesn't have JNE with immediate in all verifiers, use JEQ to skip
        // Actually it does: BPF_JMP | BPF_JNE | BPF_K
        let jump_idx = self.builder.len();
        self.builder.push(EbpfInsn::new(
            opcode::BPF_JMP | opcode::BPF_JNE | opcode::BPF_K,
            ebpf_cond.as_u8(),
            0,
            0, // Placeholder offset - will be fixed up
            0, // Compare against 0
        ));

        // Record this jump for fixup
        self.pending_jumps.push(PendingJump {
            ebpf_insn_idx: jump_idx,
            target_ir_idx,
        });

        Ok(())
    }

    /// Compile unconditional jump
    fn compile_jump(&mut self, target_ir_idx: usize) -> Result<(), CompileError> {
        let jump_idx = self.builder.len();
        self.builder.push(EbpfInsn::jump(0)); // Placeholder offset

        // Record this jump for fixup
        self.pending_jumps.push(PendingJump {
            ebpf_insn_idx: jump_idx,
            target_ir_idx,
        });

        Ok(())
    }

    /// Compile a command call - maps known commands to BPF helpers
    fn compile_call(&mut self, decl_id: DeclId, src_dst: RegId) -> Result<(), CompileError> {
        // Look up the command name
        let decl = self.engine_state.get_decl(decl_id);
        let cmd_name = decl.name();

        // Map known commands to BPF helpers
        match cmd_name {
            "bpf-pid" | "bpf pid" => {
                // bpf_get_current_pid_tgid() returns (tgid << 32) | pid
                // We'll return the full value and let user extract pid with bit ops
                self.builder.push(EbpfInsn::call(BpfHelper::GetCurrentPidTgid));
                // Result is in R0, move to destination register
                let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
                if ebpf_dst.as_u8() != EbpfReg::R0.as_u8() {
                    self.builder.push(EbpfInsn::mov64_reg(ebpf_dst, EbpfReg::R0));
                }
                Ok(())
            }
            "bpf-tgid" | "bpf tgid" => {
                // bpf_get_current_pid_tgid() returns (tgid << 32) | pid
                // TGID is in the upper 32 bits - this is the "process ID" users expect
                self.builder.push(EbpfInsn::call(BpfHelper::GetCurrentPidTgid));
                // Right-shift by 32 to get the TGID
                self.builder.push(EbpfInsn::rsh64_imm(EbpfReg::R0, 32));
                // Result is in R0, move to destination register
                let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
                if ebpf_dst.as_u8() != EbpfReg::R0.as_u8() {
                    self.builder.push(EbpfInsn::mov64_reg(ebpf_dst, EbpfReg::R0));
                }
                Ok(())
            }
            "bpf-uid" | "bpf uid" => {
                // bpf_get_current_uid_gid() returns (uid << 32) | gid
                self.builder.push(EbpfInsn::call(BpfHelper::GetCurrentUidGid));
                let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
                if ebpf_dst.as_u8() != EbpfReg::R0.as_u8() {
                    self.builder.push(EbpfInsn::mov64_reg(ebpf_dst, EbpfReg::R0));
                }
                Ok(())
            }
            "bpf-ktime" | "bpf ktime" => {
                // bpf_ktime_get_ns() returns kernel time in nanoseconds
                self.builder.push(EbpfInsn::call(BpfHelper::KtimeGetNs));
                let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
                if ebpf_dst.as_u8() != EbpfReg::R0.as_u8() {
                    self.builder.push(EbpfInsn::mov64_reg(ebpf_dst, EbpfReg::R0));
                }
                Ok(())
            }
            "bpf-emit" | "bpf emit" => {
                self.compile_bpf_emit(src_dst)
            }
            "bpf-emit-comm" | "bpf emit-comm" => {
                self.compile_bpf_emit_comm(src_dst)
            }
            "bpf-comm" | "bpf comm" => {
                self.compile_bpf_comm(src_dst)
            }
            "bpf-count" | "bpf count" => {
                self.compile_bpf_count(src_dst)
            }
            "bpf-filter-pid" | "bpf filter-pid" => {
                self.compile_bpf_filter_pid()
            }
            "bpf-filter-comm" | "bpf filter-comm" => {
                self.compile_bpf_filter_comm()
            }
            "bpf-arg" | "bpf arg" => {
                self.compile_bpf_arg(src_dst)
            }
            "bpf-retval" | "bpf retval" => {
                self.compile_bpf_retval(src_dst)
            }
            "bpf-read-str" | "bpf read-str" => {
                self.compile_bpf_read_str(src_dst, false)
            }
            "bpf-read-user-str" | "bpf read-user-str" => {
                self.compile_bpf_read_str(src_dst, true)
            }
            _ => Err(CompileError::UnsupportedInstruction(
                format!("Call to unsupported command: {}", cmd_name)
            )),
        }
    }

    /// Compile bpf-emit-comm: emit the full process name (16 bytes) to perf buffer
    ///
    /// This combines bpf_get_current_comm + bpf_perf_event_output to emit
    /// the full TASK_COMM_LEN string, not just 8 bytes.
    fn compile_bpf_emit_comm(&mut self, src_dst: RegId) -> Result<(), CompileError> {
        // Allocate the destination register (we'll store 0 as the "return value")
        let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
        // Mark that we need the perf event map
        self.needs_perf_map = true;

        // Allocate 16 bytes on stack for TASK_COMM_LEN
        let comm_stack_offset = self.stack_offset - 16;
        self.stack_offset -= 16;

        // Call bpf_get_current_comm(buf, 16)
        // R1 = pointer to buffer on stack
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R1, comm_stack_offset as i32));
        // R2 = size (16 = TASK_COMM_LEN)
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R2, 16));
        // Call bpf_get_current_comm
        self.builder.push(EbpfInsn::call(BpfHelper::GetCurrentComm));

        // Now emit the 16-byte comm to perf buffer
        // bpf_perf_event_output(ctx, map, flags, data, size)

        // R2 = map fd (load with relocation)
        let reloc_offset = self.builder.len() * 8;
        let [insn1, insn2] = EbpfInsn::ld_map_fd(EbpfReg::R2);
        self.builder.push(insn1);
        self.builder.push(insn2);
        self.relocations.push(MapRelocation {
            insn_offset: reloc_offset,
            map_name: PERF_MAP_NAME.to_string(),
        });

        // R3 = flags (BPF_F_CURRENT_CPU = 0xFFFFFFFF)
        self.builder.push(EbpfInsn::mov32_imm(EbpfReg::R3, -1));

        // R4 = pointer to comm data on stack
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R4, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R4, comm_stack_offset as i32));

        // R5 = size (16 bytes)
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R5, 16));

        // R1 = ctx (restore from R9 where we saved it at program start)
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R9));

        // Call bpf_perf_event_output
        self.builder.push(EbpfInsn::call(BpfHelper::PerfEventOutput));

        // Set destination register to 0 (success indicator)
        self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, 0));

        Ok(())
    }

    /// Compile bpf-count: increment a counter for the input key
    ///
    /// Uses a hash map to count occurrences by key. The input value is used
    /// as the key, and the counter is atomically incremented.
    fn compile_bpf_count(&mut self, src_dst: RegId) -> Result<(), CompileError> {
        // Mark that we need the counter map
        self.needs_counter_map = true;

        let ebpf_src = self.reg_alloc.get(src_dst)?;

        // Allocate stack space for key and value
        // Key: 8 bytes (i64)
        // Value: 8 bytes (i64)
        let key_stack_offset = self.stack_offset - 8;
        let value_stack_offset = self.stack_offset - 16;
        self.stack_offset -= 16;

        // Store the key to stack
        self.builder.push(EbpfInsn::stxdw(EbpfReg::R10, key_stack_offset, ebpf_src));

        // Step 1: Try to look up existing value
        // R1 = map (will be relocated)
        let lookup_reloc_offset = self.builder.len() * 8;
        let [insn1, insn2] = EbpfInsn::ld_map_fd(EbpfReg::R1);
        self.builder.push(insn1);
        self.builder.push(insn2);
        self.relocations.push(MapRelocation {
            insn_offset: lookup_reloc_offset,
            map_name: COUNTER_MAP_NAME.to_string(),
        });

        // R2 = pointer to key
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R2, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R2, key_stack_offset as i32));

        // Call bpf_map_lookup_elem
        self.builder.push(EbpfInsn::call(BpfHelper::MapLookupElem));

        // R0 = pointer to value or NULL
        // If NULL, initialize to 0; otherwise, increment
        // jeq r0, 0, +3 (skip to initialize if NULL)
        self.builder.push(EbpfInsn::jeq_imm(EbpfReg::R0, 0, 4));

        // Value exists - load it, increment, store back
        // Load current value: r1 = *r0
        self.builder.push(EbpfInsn::ldxdw(EbpfReg::R1, EbpfReg::R0, 0));
        // Increment: r1 += 1
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R1, 1));
        // Store back: *r0 = r1
        self.builder.push(EbpfInsn::stxdw(EbpfReg::R0, 0, EbpfReg::R1));
        // Jump to end - skip the 10 instructions of the initialization path:
        // mov + stxdw + ld_map_fd(2) + mov + add + mov + add + mov + call = 10
        self.builder.push(EbpfInsn::jump(10));

        // Value doesn't exist - initialize to 1 and insert
        // Store 1 to value slot on stack
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R1, 1));
        self.builder.push(EbpfInsn::stxdw(EbpfReg::R10, value_stack_offset, EbpfReg::R1));

        // R1 = map (reload for update)
        let update_reloc_offset = self.builder.len() * 8;
        let [insn1, insn2] = EbpfInsn::ld_map_fd(EbpfReg::R1);
        self.builder.push(insn1);
        self.builder.push(insn2);
        self.relocations.push(MapRelocation {
            insn_offset: update_reloc_offset,
            map_name: COUNTER_MAP_NAME.to_string(),
        });

        // R2 = pointer to key
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R2, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R2, key_stack_offset as i32));

        // R3 = pointer to value
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R3, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R3, value_stack_offset as i32));

        // R4 = flags (0 = BPF_ANY)
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R4, 0));

        // Call bpf_map_update_elem
        self.builder.push(EbpfInsn::call(BpfHelper::MapUpdateElem));

        // End: bpf-count passes through the input value unchanged
        // (it's still in the original register)

        Ok(())
    }

    /// Compile bpf-comm: get current process name
    ///
    /// Calls bpf_get_current_comm to get the process name, then returns
    /// the first 8 bytes as an i64 for easy comparison/emission.
    fn compile_bpf_comm(&mut self, src_dst: RegId) -> Result<(), CompileError> {
        // Track that this register contains a comm value
        self.register_types.insert(src_dst.get(), BpfFieldType::Comm);

        // Allocate 16 bytes on stack for TASK_COMM_LEN
        let comm_stack_offset = self.stack_offset - 16;
        self.stack_offset -= 16;

        // R1 = pointer to buffer on stack
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R1, comm_stack_offset as i32));

        // R2 = size (16 = TASK_COMM_LEN)
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R2, 16));

        // Call bpf_get_current_comm
        self.builder.push(EbpfInsn::call(BpfHelper::GetCurrentComm));

        // Load first 8 bytes from buffer into destination register
        let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
        self.builder.push(EbpfInsn::ldxdw(ebpf_dst, EbpfReg::R10, comm_stack_offset));

        Ok(())
    }

    /// Compile bpf-emit: output a value to the perf event buffer
    ///
    /// This uses bpf_perf_event_output to send a 64-bit value to userspace.
    /// The event structure is simple: just the 64-bit value.
    /// If the input is a record, emits all fields as a structured event.
    fn compile_bpf_emit(&mut self, src_dst: RegId) -> Result<(), CompileError> {
        // Check if the source is a record
        if let Some(record) = self.record_builders.remove(&src_dst.get()) {
            return self.compile_bpf_emit_record(src_dst, record);
        }

        // Mark that we need the perf event map
        self.needs_perf_map = true;

        let ebpf_src = self.reg_alloc.get(src_dst)?;

        // Allocate stack space for the event data (8 bytes for u64)
        let event_stack_offset = self.stack_offset;
        self.stack_offset -= 8;

        // Store the value to the stack for bpf_perf_event_output
        self.builder.push(EbpfInsn::stxdw(
            EbpfReg::R10,
            event_stack_offset,
            ebpf_src,
        ));

        // bpf_perf_event_output(ctx, map, flags, data, size)
        // R1 = ctx (pt_regs pointer - should still be valid if called early)
        // R2 = map (will be relocated by loader)
        // R3 = flags (BPF_F_CURRENT_CPU = 0xFFFFFFFF)
        // R4 = data pointer (stack address)
        // R5 = data size (8 bytes)

        // R2 = map fd (load with relocation)
        let reloc_offset = self.builder.len() * 8; // Byte offset
        let [insn1, insn2] = EbpfInsn::ld_map_fd(EbpfReg::R2);
        self.builder.push(insn1);
        self.builder.push(insn2);

        // Record relocation
        self.relocations.push(MapRelocation {
            insn_offset: reloc_offset,
            map_name: PERF_MAP_NAME.to_string(),
        });

        // R3 = flags (BPF_F_CURRENT_CPU = 0xFFFFFFFF)
        // Use mov32 which zeros the upper 32 bits and sets lower 32 bits
        self.builder.push(EbpfInsn::mov32_imm(EbpfReg::R3, -1));  // R3 = 0x00000000FFFFFFFF

        // R4 = pointer to data on stack
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R4, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R4, event_stack_offset as i32));

        // R5 = size (8 bytes)
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R5, 8));

        // R1 = ctx (restore from R9 where we saved it at program start)
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R9));

        // Call bpf_perf_event_output
        self.builder.push(EbpfInsn::call(BpfHelper::PerfEventOutput));

        // bpf-emit returns the original value for chaining
        // The value is still in ebpf_src register (we only copied it to stack)

        Ok(())
    }

    /// Compile bpf-filter-pid: exit early if current TGID doesn't match
    ///
    /// Gets the first pushed positional argument (target PID) and compares
    /// with the current TGID. If they don't match, exits the program early.
    fn compile_bpf_filter_pid(&mut self) -> Result<(), CompileError> {
        // Get the target PID from pushed arguments
        let arg_reg = self.pushed_args.pop().ok_or_else(|| {
            CompileError::UnsupportedInstruction("bpf-filter-pid requires a PID argument".into())
        })?;

        // Get the target PID value (should already be loaded in a register)
        let target_reg = self.reg_alloc.get(arg_reg)?;

        // Get current TGID
        self.builder.push(EbpfInsn::call(BpfHelper::GetCurrentPidTgid));
        // Right-shift by 32 to get the TGID
        self.builder.push(EbpfInsn::rsh64_imm(EbpfReg::R0, 32));

        // Compare R0 (current TGID) with target
        // If equal, continue; if not equal, exit with 0
        // jne r0, target_reg, +2 (skip to exit)
        self.builder.push(EbpfInsn::jeq_reg(EbpfReg::R0, target_reg, 2));

        // Not matching - exit early
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R0, 0));
        self.builder.push(EbpfInsn::exit());

        // Matching - continue execution (fall through)
        Ok(())
    }

    /// Compile bpf-filter-comm: exit early if current comm doesn't match
    ///
    /// Gets the first pushed positional argument (target comm as i64) and
    /// compares with the first 8 bytes of current comm. If they don't match,
    /// exits the program early.
    fn compile_bpf_filter_comm(&mut self) -> Result<(), CompileError> {
        // Get the target comm from pushed arguments
        let arg_reg = self.pushed_args.pop().ok_or_else(|| {
            CompileError::UnsupportedInstruction("bpf-filter-comm requires a comm argument".into())
        })?;

        // Get the target comm value (should already be loaded in a register)
        let target_reg = self.reg_alloc.get(arg_reg)?;

        // Get current comm (first 8 bytes)
        // Allocate 16 bytes on stack for TASK_COMM_LEN
        let comm_stack_offset = self.stack_offset - 16;
        self.stack_offset -= 16;

        // R1 = pointer to buffer on stack
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R1, comm_stack_offset as i32));

        // R2 = size (16 = TASK_COMM_LEN)
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R2, 16));

        // Call bpf_get_current_comm
        self.builder.push(EbpfInsn::call(BpfHelper::GetCurrentComm));

        // Load first 8 bytes from buffer into R0
        self.builder.push(EbpfInsn::ldxdw(EbpfReg::R0, EbpfReg::R10, comm_stack_offset));

        // Compare R0 (current comm first 8 bytes) with target
        // If equal, continue; if not equal, exit with 0
        self.builder.push(EbpfInsn::jeq_reg(EbpfReg::R0, target_reg, 2));

        // Not matching - exit early
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R0, 0));
        self.builder.push(EbpfInsn::exit());

        // Matching - continue execution (fall through)
        Ok(())
    }

    /// Compile bpf-arg: read a function argument from pt_regs
    ///
    /// The argument index is passed as a positional argument.
    /// Reads from the context pointer (saved in R9) at the appropriate offset.
    fn compile_bpf_arg(&mut self, src_dst: RegId) -> Result<(), CompileError> {
        // Get the argument index from pushed arguments
        let arg_reg = self.pushed_args.pop().ok_or_else(|| {
            CompileError::UnsupportedInstruction("bpf-arg requires an index argument".into())
        })?;

        // Look up the compile-time literal value for the index
        let index = self.literal_values.get(&arg_reg.get()).copied().ok_or_else(|| {
            CompileError::UnsupportedInstruction(
                "bpf-arg index must be a compile-time constant (literal integer)".into()
            )
        })?;

        // Validate the index
        let max_args = pt_regs_offsets::ARG_OFFSETS.len();
        if index < 0 || index as usize >= max_args {
            return Err(CompileError::UnsupportedInstruction(
                format!("bpf-arg index {} out of range (0-{})", index, max_args - 1)
            ));
        }

        // Get the offset for this argument
        let offset = pt_regs_offsets::ARG_OFFSETS[index as usize];

        // Allocate destination register
        let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;

        // ldxdw dst, [r9 + offset] - load 64-bit value from ctx
        self.builder.push(EbpfInsn::ldxdw(ebpf_dst, EbpfReg::R9, offset));

        Ok(())
    }

    /// Compile bpf-retval: read the return value from pt_regs (for kretprobe)
    ///
    /// Reads the return value register from the context pointer.
    fn compile_bpf_retval(&mut self, src_dst: RegId) -> Result<(), CompileError> {
        // Allocate destination register
        let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;

        // Read the return value from context (R9 has the ctx pointer)
        // On x86_64, return value is in rax at offset 80
        let offset = pt_regs_offsets::RETVAL_OFFSET;

        // ldxdw dst, [r9 + offset] - load 64-bit value from ctx
        self.builder.push(EbpfInsn::ldxdw(ebpf_dst, EbpfReg::R9, offset));

        Ok(())
    }

    /// Compile bpf-read-str / bpf-read-user-str: read a string and emit it
    ///
    /// Takes a pointer from the pipeline input, reads up to 128 bytes of
    /// null-terminated string from memory, and emits to perf buffer.
    ///
    /// If `user_space` is true, reads from user-space memory (for syscall args).
    /// If `user_space` is false, reads from kernel memory.
    fn compile_bpf_read_str(&mut self, src_dst: RegId, user_space: bool) -> Result<(), CompileError> {
        // Mark that we need the perf event map
        self.needs_perf_map = true;

        // Get the source pointer from the input register
        let src_ptr = self.reg_alloc.get(src_dst)?;

        // Allocate stack space for the string buffer (128 bytes max)
        const STR_BUF_SIZE: i16 = 128;
        let str_stack_offset = self.stack_offset - STR_BUF_SIZE;
        self.stack_offset -= STR_BUF_SIZE;

        // Call bpf_probe_read_{kernel,user}_str(dst, size, unsafe_ptr)
        // R1 = dst (stack buffer)
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R1, str_stack_offset as i32));

        // R2 = size
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R2, STR_BUF_SIZE as i32));

        // R3 = src pointer (from input)
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R3, src_ptr));

        // Call appropriate helper based on memory type
        let helper = if user_space {
            BpfHelper::ProbeReadUserStr
        } else {
            BpfHelper::ProbeReadKernelStr
        };
        self.builder.push(EbpfInsn::call(helper));

        // R0 now contains the number of bytes read (including null terminator)
        // or negative error code

        // Now emit the string to perf buffer
        // bpf_perf_event_output(ctx, map, flags, data, size)

        // R2 = map (will be relocated)
        let reloc_offset = self.builder.len() * 8;
        let [insn1, insn2] = EbpfInsn::ld_map_fd(EbpfReg::R2);
        self.builder.push(insn1);
        self.builder.push(insn2);
        self.relocations.push(MapRelocation {
            insn_offset: reloc_offset,
            map_name: PERF_MAP_NAME.to_string(),
        });

        // R3 = flags (BPF_F_CURRENT_CPU)
        self.builder.push(EbpfInsn::mov32_imm(EbpfReg::R3, -1));

        // R4 = pointer to data on stack
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R4, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R4, str_stack_offset as i32));

        // R5 = size (use the return value from probe_read_kernel_str if positive,
        // otherwise use full buffer size)
        // For simplicity, just use the full buffer size
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R5, STR_BUF_SIZE as i32));

        // R1 = ctx (restore from R9)
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R9));

        // Call bpf_perf_event_output
        self.builder.push(EbpfInsn::call(BpfHelper::PerfEventOutput));

        // Set result to 0 (success indicator)
        let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
        self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, 0));

        Ok(())
    }

    /// Compile RecordInsert: add a field to a record being built
    ///
    /// This immediately stores the field value to the stack to preserve it.
    fn compile_record_insert(&mut self, src_dst: RegId, key: RegId, val: RegId) -> Result<(), CompileError> {
        // Get the field name from the key register's literal string
        let field_name = self.literal_strings.get(&key.get()).cloned().ok_or_else(|| {
            CompileError::UnsupportedInstruction(
                "Record field name must be a literal string".into()
            )
        })?;

        // Determine the field type from the value register
        let field_type = self.register_types.get(&val.get()).copied().unwrap_or(BpfFieldType::Int);
        let field_size = field_type.size() as i16;

        // Get the eBPF register containing the value
        // Use get_or_alloc in case the value comes from a literal that wasn't separately allocated
        let ebpf_val = self.reg_alloc.get_or_alloc(val)?;

        // Allocate stack space for this field and store immediately
        let field_stack_offset = self.stack_offset - field_size;
        self.stack_offset -= field_size;

        // Store the value to the stack based on field type
        match field_type {
            BpfFieldType::Int => {
                self.builder.push(EbpfInsn::stxdw(EbpfReg::R10, field_stack_offset, ebpf_val));
            }
            BpfFieldType::Comm => {
                // Store 8-byte value we have (first 8 bytes of comm)
                self.builder.push(EbpfInsn::stxdw(EbpfReg::R10, field_stack_offset, ebpf_val));
                // Zero-fill remaining 8 bytes
                self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R0, 0));
                self.builder.push(EbpfInsn::stxdw(EbpfReg::R10, field_stack_offset + 8, EbpfReg::R0));
            }
            BpfFieldType::String => {
                // Store 8-byte value we have
                self.builder.push(EbpfInsn::stxdw(EbpfReg::R10, field_stack_offset, ebpf_val));
                // Zero-fill remaining bytes (simplified)
                self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R0, 0));
                for i in 1..16 {
                    self.builder.push(EbpfInsn::stxdw(EbpfReg::R10, field_stack_offset + (i * 8), EbpfReg::R0));
                }
            }
        }

        // Get or create the record builder for the destination register
        let record = self.record_builders.entry(src_dst.get()).or_insert_with(|| RecordBuilder {
            fields: Vec::new(),
            current_offset: 0,
            base_offset: field_stack_offset, // First field determines base
        });

        // Update base_offset if this is the first field
        if record.fields.is_empty() {
            record.base_offset = field_stack_offset;
        }

        // Add the field to the record
        record.fields.push(RecordFieldBuilder {
            name: field_name,
            stack_offset: field_stack_offset,
            field_type,
        });

        Ok(())
    }

    /// Compile bpf-emit for a structured record
    ///
    /// The field values are already on the stack (stored during RecordInsert).
    /// We just need to emit them to the perf buffer.
    fn compile_bpf_emit_record(&mut self, src_dst: RegId, record: RecordBuilder) -> Result<(), CompileError> {
        self.needs_perf_map = true;

        if record.fields.is_empty() {
            // Empty record - just emit nothing
            let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
            self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, 0));
            return Ok(());
        }

        // Build schema from fields
        // Fields are stored in descending stack order (later fields have lower addresses)
        // So when we emit the buffer starting from the lowest address, we get fields in reverse order
        // We need to reverse the schema to match the actual memory layout
        let mut fields_schema = Vec::new();
        let mut offset = 0usize;

        // Iterate in reverse to match memory layout (lowest address = last field inserted)
        for field in record.fields.iter().rev() {
            let size = field.field_type.size();
            fields_schema.push(SchemaField {
                name: field.name.clone(),
                field_type: field.field_type,
                offset,
            });
            offset += size;
        }

        let total_size = offset;

        // Store the schema for the loader
        self.event_schema = Some(EventSchema {
            fields: fields_schema,
            total_size,
        });

        // The first field's stack offset is the start of our data
        // Fields are stored contiguously in reverse order on stack
        // So we need to find the lowest stack offset (most recent allocation)
        let record_start_offset = record.fields.last()
            .map(|f| f.stack_offset)
            .unwrap_or(record.base_offset);

        // Emit the record to perf buffer
        // bpf_perf_event_output(ctx, map, flags, data, size)

        // R2 = map (will be relocated)
        let reloc_offset = self.builder.len() * 8;
        let [insn1, insn2] = EbpfInsn::ld_map_fd(EbpfReg::R2);
        self.builder.push(insn1);
        self.builder.push(insn2);
        self.relocations.push(MapRelocation {
            insn_offset: reloc_offset,
            map_name: PERF_MAP_NAME.to_string(),
        });

        // R3 = flags (BPF_F_CURRENT_CPU)
        self.builder.push(EbpfInsn::mov32_imm(EbpfReg::R3, -1));

        // R4 = pointer to record on stack (use the first field's offset as start)
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R4, EbpfReg::R10));
        self.builder.push(EbpfInsn::add64_imm(EbpfReg::R4, record_start_offset as i32));

        // R5 = total record size
        self.builder.push(EbpfInsn::mov64_imm(EbpfReg::R5, total_size as i32));

        // R1 = ctx (restore from R9)
        self.builder.push(EbpfInsn::mov64_reg(EbpfReg::R1, EbpfReg::R9));

        // Call bpf_perf_event_output
        self.builder.push(EbpfInsn::call(BpfHelper::PerfEventOutput));

        // Set destination to 0
        let ebpf_dst = self.reg_alloc.get_or_alloc(src_dst)?;
        self.builder.push(EbpfInsn::mov64_imm(ebpf_dst, 0));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::ir::IrBlock;
    use std::sync::Arc;

    fn make_ir_block(instructions: Vec<Instruction>) -> IrBlock {
        IrBlock {
            instructions,
            spans: vec![],
            data: Arc::from([]),
            ast: vec![],
            comments: vec![],
            register_count: 10,
            file_count: 0,
        }
    }

    #[test]
    fn test_compile_return_zero() {
        let ir = make_ir_block(vec![
            Instruction::LoadLiteral {
                dst: RegId::new(0),
                lit: Literal::Int(0),
            },
            Instruction::Return {
                src: RegId::new(0),
            },
        ]);

        let bytecode = IrToEbpfCompiler::compile_no_calls(&ir).unwrap();
        // Should have: mov r6, 0; mov r0, r6; exit
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_add() {
        let ir = make_ir_block(vec![
            Instruction::LoadLiteral {
                dst: RegId::new(0),
                lit: Literal::Int(1),
            },
            Instruction::LoadLiteral {
                dst: RegId::new(1),
                lit: Literal::Int(2),
            },
            Instruction::BinaryOp {
                lhs_dst: RegId::new(0),
                op: Operator::Math(Math::Add),
                rhs: RegId::new(1),
            },
            Instruction::Return {
                src: RegId::new(0),
            },
        ]);

        let bytecode = IrToEbpfCompiler::compile_no_calls(&ir).unwrap();
        assert!(!bytecode.is_empty());
    }
}
