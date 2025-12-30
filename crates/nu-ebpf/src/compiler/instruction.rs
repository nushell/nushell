//! eBPF instruction encoding
//!
//! eBPF instructions are 64-bit fixed-length, encoded as:
//! ```text
//! opcode:8 src_reg:4 dst_reg:4 offset:16 imm:32
//! ```
//!
//! Some instructions (like 64-bit immediate loads) use two 64-bit slots.

/// eBPF register identifiers (r0-r10)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EbpfReg {
    /// Return value from functions, exit value for eBPF program
    R0 = 0,
    /// First argument to BPF helpers, also context pointer
    R1 = 1,
    /// Second argument to BPF helpers
    R2 = 2,
    /// Third argument to BPF helpers
    R3 = 3,
    /// Fourth argument to BPF helpers
    R4 = 4,
    /// Fifth argument to BPF helpers
    R5 = 5,
    /// Callee-saved register
    R6 = 6,
    /// Callee-saved register
    R7 = 7,
    /// Callee-saved register
    R8 = 8,
    /// Callee-saved register
    R9 = 9,
    /// Frame pointer (read-only)
    R10 = 10,
}

impl EbpfReg {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// BPF helper function numbers
///
/// These are the kernel helper functions that eBPF programs can call.
/// See: https://man7.org/linux/man-pages/man7/bpf-helpers.7.html
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum BpfHelper {
    /// void *bpf_map_lookup_elem(map, key)
    MapLookupElem = 1,
    /// int bpf_map_update_elem(map, key, value, flags)
    MapUpdateElem = 2,
    /// int bpf_map_delete_elem(map, key)
    MapDeleteElem = 3,
    /// int bpf_probe_read(dst, size, src)
    ProbeRead = 4,
    /// u64 bpf_ktime_get_ns(void)
    KtimeGetNs = 5,
    /// int bpf_trace_printk(fmt, fmt_size, ...)
    TracePrintk = 6,
    /// u64 bpf_get_current_pid_tgid(void)
    GetCurrentPidTgid = 14,
    /// u64 bpf_get_current_uid_gid(void)
    GetCurrentUidGid = 15,
    /// int bpf_get_current_comm(buf, size)
    GetCurrentComm = 16,
    /// int bpf_perf_event_output(ctx, map, flags, data, size)
    PerfEventOutput = 25,
    /// long bpf_probe_read_user_str(dst, size, unsafe_ptr)
    ProbeReadUserStr = 114,
    /// long bpf_probe_read_kernel_str(dst, size, unsafe_ptr)
    ProbeReadKernelStr = 115,
}

/// eBPF instruction opcodes
pub mod opcode {
    // Instruction classes (3 bits)
    pub const BPF_LD: u8 = 0x00;
    pub const BPF_LDX: u8 = 0x01;
    pub const BPF_ST: u8 = 0x02;
    pub const BPF_STX: u8 = 0x03;
    pub const BPF_ALU: u8 = 0x04;
    pub const BPF_JMP: u8 = 0x05;
    pub const BPF_JMP32: u8 = 0x06;
    pub const BPF_ALU64: u8 = 0x07;

    // Size modifiers (2 bits)
    pub const BPF_W: u8 = 0x00; // 32-bit
    pub const BPF_H: u8 = 0x08; // 16-bit
    pub const BPF_B: u8 = 0x10; // 8-bit
    pub const BPF_DW: u8 = 0x18; // 64-bit

    // Source modifiers
    pub const BPF_K: u8 = 0x00; // Immediate
    pub const BPF_X: u8 = 0x08; // Register

    // ALU operations (4 bits, shifted left by 4)
    pub const BPF_ADD: u8 = 0x00;
    pub const BPF_SUB: u8 = 0x10;
    pub const BPF_MUL: u8 = 0x20;
    pub const BPF_DIV: u8 = 0x30;
    pub const BPF_OR: u8 = 0x40;
    pub const BPF_AND: u8 = 0x50;
    pub const BPF_LSH: u8 = 0x60;
    pub const BPF_RSH: u8 = 0x70;
    pub const BPF_NEG: u8 = 0x80;
    pub const BPF_MOD: u8 = 0x90;
    pub const BPF_XOR: u8 = 0xa0;
    pub const BPF_MOV: u8 = 0xb0;
    pub const BPF_ARSH: u8 = 0xc0; // Arithmetic right shift

    // Jump operations
    pub const BPF_JA: u8 = 0x00; // Jump always
    pub const BPF_JEQ: u8 = 0x10; // Jump if equal
    pub const BPF_JGT: u8 = 0x20; // Jump if greater than
    pub const BPF_JGE: u8 = 0x30; // Jump if greater or equal
    pub const BPF_JSET: u8 = 0x40; // Jump if set (bitwise AND)
    pub const BPF_JNE: u8 = 0x50; // Jump if not equal
    pub const BPF_JSGT: u8 = 0x60; // Jump if signed greater than
    pub const BPF_JSGE: u8 = 0x70; // Jump if signed greater or equal
    pub const BPF_CALL: u8 = 0x80; // Function call
    pub const BPF_EXIT: u8 = 0x90; // Exit program
    pub const BPF_JLT: u8 = 0xa0; // Jump if less than
    pub const BPF_JLE: u8 = 0xb0; // Jump if less or equal
    pub const BPF_JSLT: u8 = 0xc0; // Jump if signed less than
    pub const BPF_JSLE: u8 = 0xd0; // Jump if signed less or equal

    // Memory modes
    pub const BPF_IMM: u8 = 0x00;
    pub const BPF_ABS: u8 = 0x20;
    pub const BPF_IND: u8 = 0x40;
    pub const BPF_MEM: u8 = 0x60;

    // Composite opcodes for common operations
    pub const MOV64_IMM: u8 = BPF_ALU64 | BPF_MOV | BPF_K; // 0xb7
    pub const MOV64_REG: u8 = BPF_ALU64 | BPF_MOV | BPF_X; // 0xbf
    pub const ADD64_IMM: u8 = BPF_ALU64 | BPF_ADD | BPF_K; // 0x07
    pub const ADD64_REG: u8 = BPF_ALU64 | BPF_ADD | BPF_X; // 0x0f
    pub const SUB64_IMM: u8 = BPF_ALU64 | BPF_SUB | BPF_K; // 0x17
    pub const SUB64_REG: u8 = BPF_ALU64 | BPF_SUB | BPF_X; // 0x1f
    pub const MUL64_IMM: u8 = BPF_ALU64 | BPF_MUL | BPF_K; // 0x27
    pub const MUL64_REG: u8 = BPF_ALU64 | BPF_MUL | BPF_X; // 0x2f
    pub const DIV64_IMM: u8 = BPF_ALU64 | BPF_DIV | BPF_K; // 0x37
    pub const DIV64_REG: u8 = BPF_ALU64 | BPF_DIV | BPF_X; // 0x3f
    pub const MOD64_IMM: u8 = BPF_ALU64 | BPF_MOD | BPF_K; // 0x97
    pub const MOD64_REG: u8 = BPF_ALU64 | BPF_MOD | BPF_X; // 0x9f
    pub const OR64_IMM: u8 = BPF_ALU64 | BPF_OR | BPF_K; // 0x47
    pub const OR64_REG: u8 = BPF_ALU64 | BPF_OR | BPF_X; // 0x4f
    pub const AND64_IMM: u8 = BPF_ALU64 | BPF_AND | BPF_K; // 0x57
    pub const AND64_REG: u8 = BPF_ALU64 | BPF_AND | BPF_X; // 0x5f
    pub const XOR64_IMM: u8 = BPF_ALU64 | BPF_XOR | BPF_K; // 0xa7
    pub const XOR64_REG: u8 = BPF_ALU64 | BPF_XOR | BPF_X; // 0xaf
    pub const LSH64_IMM: u8 = BPF_ALU64 | BPF_LSH | BPF_K; // 0x67
    pub const LSH64_REG: u8 = BPF_ALU64 | BPF_LSH | BPF_X; // 0x6f
    pub const RSH64_IMM: u8 = BPF_ALU64 | BPF_RSH | BPF_K; // 0x77
    pub const RSH64_REG: u8 = BPF_ALU64 | BPF_RSH | BPF_X; // 0x7f
    pub const CALL: u8 = BPF_JMP | BPF_CALL; // 0x85
    pub const EXIT: u8 = BPF_JMP | BPF_EXIT; // 0x95
    pub const LD_DW_IMM: u8 = BPF_LD | BPF_DW | BPF_IMM; // 0x18 (64-bit immediate load)
}

/// A single eBPF instruction (64-bit)
#[derive(Debug, Clone, Copy)]
pub struct EbpfInsn {
    /// Operation code
    pub opcode: u8,
    /// Destination register (4 bits, lower nibble)
    pub dst_reg: u8,
    /// Source register (4 bits, upper nibble)
    pub src_reg: u8,
    /// Signed offset for memory/branch operations
    pub offset: i16,
    /// Signed immediate value
    pub imm: i32,
}

impl EbpfInsn {
    /// Create a new instruction
    pub const fn new(opcode: u8, dst_reg: u8, src_reg: u8, offset: i16, imm: i32) -> Self {
        Self {
            opcode,
            dst_reg,
            src_reg,
            offset,
            imm,
        }
    }

    /// Encode the instruction to 8 bytes (little-endian)
    pub fn encode(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.opcode;
        bytes[1] = (self.src_reg << 4) | (self.dst_reg & 0x0f);
        bytes[2..4].copy_from_slice(&self.offset.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.imm.to_le_bytes());
        bytes
    }

    // ===== Instruction builders =====

    /// MOV64 dst, imm - Load 32-bit immediate into 64-bit register (sign-extends)
    pub const fn mov64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::MOV64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// MOV32 dst, imm - Load 32-bit immediate into lower 32 bits of register (zeros upper bits)
    pub const fn mov32_imm(dst: EbpfReg, imm: i32) -> Self {
        // BPF_ALU (32-bit) | BPF_MOV | BPF_K = 0x04 | 0xb0 | 0x00 = 0xb4
        Self::new(0xb4, dst.as_u8(), 0, 0, imm)
    }

    /// MOV64 dst, src - Copy register
    pub const fn mov64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::MOV64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// ADD64 dst, imm - Add immediate to register
    pub const fn add64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::ADD64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// ADD64 dst, src - Add register to register
    pub const fn add64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::ADD64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// SUB64 dst, imm - Subtract immediate from register
    pub const fn sub64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::SUB64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// SUB64 dst, src - Subtract register from register
    pub const fn sub64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::SUB64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// MUL64 dst, imm - Multiply register by immediate
    pub const fn mul64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::MUL64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// MUL64 dst, src - Multiply register by register
    pub const fn mul64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::MUL64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// DIV64 dst, imm - Divide register by immediate
    pub const fn div64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::DIV64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// DIV64 dst, src - Divide register by register
    pub const fn div64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::DIV64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// MOD64 dst, imm - Modulo register by immediate
    pub const fn mod64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::MOD64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// MOD64 dst, src - Modulo register by register
    pub const fn mod64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::MOD64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// OR64 dst, imm - Bitwise OR register with immediate
    pub const fn or64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::OR64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// OR64 dst, src - Bitwise OR register with register
    pub const fn or64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::OR64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// AND64 dst, imm - Bitwise AND register with immediate
    pub const fn and64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::AND64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// AND64 dst, src - Bitwise AND register with register
    pub const fn and64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::AND64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// XOR64 dst, imm - Bitwise XOR register with immediate
    pub const fn xor64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::XOR64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// XOR64 dst, src - Bitwise XOR register with register
    pub const fn xor64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::XOR64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// LSH64 dst, imm - Left shift register by immediate
    pub const fn lsh64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::LSH64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// LSH64 dst, src - Left shift register by register
    pub const fn lsh64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::LSH64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// RSH64 dst, imm - Right shift register by immediate
    pub const fn rsh64_imm(dst: EbpfReg, imm: i32) -> Self {
        Self::new(opcode::RSH64_IMM, dst.as_u8(), 0, 0, imm)
    }

    /// RSH64 dst, src - Right shift register by register
    pub const fn rsh64_reg(dst: EbpfReg, src: EbpfReg) -> Self {
        Self::new(opcode::RSH64_REG, dst.as_u8(), src.as_u8(), 0, 0)
    }

    /// CALL helper - Call a BPF helper function
    pub const fn call(helper: BpfHelper) -> Self {
        Self::new(opcode::CALL, 0, 0, 0, helper as i32)
    }

    /// EXIT - Exit the eBPF program (return value in r0)
    pub const fn exit() -> Self {
        Self::new(opcode::EXIT, 0, 0, 0, 0)
    }

    /// JA offset - Unconditional jump (offset is relative to next instruction)
    pub const fn jump(offset: i16) -> Self {
        Self::new(opcode::BPF_JMP | opcode::BPF_JA, 0, 0, offset, 0)
    }

    /// JNE dst, src, offset - Jump if dst != src (unsigned)
    pub const fn jne_reg(dst: EbpfReg, src: EbpfReg, offset: i16) -> Self {
        Self::new(opcode::BPF_JMP | opcode::BPF_JNE | opcode::BPF_X, dst.as_u8(), src.as_u8(), offset, 0)
    }

    /// JEQ dst, imm, offset - Jump if dst == imm
    pub const fn jeq_imm(dst: EbpfReg, imm: i32, offset: i16) -> Self {
        Self::new(opcode::BPF_JMP | opcode::BPF_JEQ | opcode::BPF_K, dst.as_u8(), 0, offset, imm)
    }

    /// JEQ dst, src, offset - Jump if dst == src (register comparison)
    pub const fn jeq_reg(dst: EbpfReg, src: EbpfReg, offset: i16) -> Self {
        Self::new(opcode::BPF_JMP | opcode::BPF_JEQ | opcode::BPF_X, dst.as_u8(), src.as_u8(), offset, 0)
    }

    /// NEG64 dst - Negate register (dst = -dst)
    pub const fn neg64(dst: EbpfReg) -> Self {
        Self::new(opcode::BPF_ALU64 | opcode::BPF_NEG, dst.as_u8(), 0, 0, 0)
    }

    /// STXDW [dst+off], src - Store 64-bit value from register to memory
    pub const fn stxdw(dst: EbpfReg, offset: i16, src: EbpfReg) -> Self {
        Self::new(
            opcode::BPF_STX | opcode::BPF_DW | opcode::BPF_MEM,
            dst.as_u8(),
            src.as_u8(),
            offset,
            0,
        )
    }

    /// STXW [dst+off], src - Store 32-bit value from register to memory
    pub const fn stxw(dst: EbpfReg, offset: i16, src: EbpfReg) -> Self {
        Self::new(
            opcode::BPF_STX | opcode::BPF_W | opcode::BPF_MEM,
            dst.as_u8(),
            src.as_u8(),
            offset,
            0,
        )
    }

    /// LDXDW dst, [src+off] - Load 64-bit value from memory to register
    pub const fn ldxdw(dst: EbpfReg, src: EbpfReg, offset: i16) -> Self {
        Self::new(
            opcode::BPF_LDX | opcode::BPF_DW | opcode::BPF_MEM,
            dst.as_u8(),
            src.as_u8(),
            offset,
            0,
        )
    }

    /// LD_MAP_FD - Load map file descriptor (pseudo instruction, needs relocation)
    /// This creates a 16-byte instruction (two slots) that will be patched by the loader
    pub fn ld_map_fd(dst: EbpfReg) -> [Self; 2] {
        [
            Self::new(
                opcode::LD_DW_IMM,
                dst.as_u8(),
                1, // src_reg=1 means "load map by fd"
                0,
                0, // Will be filled by relocation
            ),
            Self::new(0, 0, 0, 0, 0), // Second half of 128-bit instruction
        ]
    }
}

/// Builder for constructing eBPF programs
#[derive(Debug, Default)]
pub struct EbpfBuilder {
    instructions: Vec<EbpfInsn>,
}

impl EbpfBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an instruction
    pub fn push(&mut self, insn: EbpfInsn) -> &mut Self {
        self.instructions.push(insn);
        self
    }

    /// Get the current instruction count
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Build the raw bytecode
    pub fn build(self) -> Vec<u8> {
        let mut bytecode = Vec::with_capacity(self.instructions.len() * 8);
        for insn in self.instructions {
            bytecode.extend_from_slice(&insn.encode());
        }
        bytecode
    }

    /// Get instructions for inspection
    pub fn instructions(&self) -> &[EbpfInsn] {
        &self.instructions
    }

    /// Set the offset field of an instruction (for fixup of jumps)
    pub fn set_offset(&mut self, idx: usize, offset: i16) {
        if let Some(insn) = self.instructions.get_mut(idx) {
            insn.offset = offset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mov64_imm_encoding() {
        let insn = EbpfInsn::mov64_imm(EbpfReg::R0, 0);
        let bytes = insn.encode();
        // opcode=0xb7, regs=0x00, offset=0x0000, imm=0x00000000
        assert_eq!(bytes, [0xb7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_mov64_imm_with_value() {
        let insn = EbpfInsn::mov64_imm(EbpfReg::R1, 42);
        let bytes = insn.encode();
        // opcode=0xb7, regs=0x01 (dst=1), offset=0x0000, imm=42
        assert_eq!(bytes, [0xb7, 0x01, 0x00, 0x00, 0x2a, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_exit_encoding() {
        let insn = EbpfInsn::exit();
        let bytes = insn.encode();
        // opcode=0x95
        assert_eq!(bytes, [0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_call_helper() {
        let insn = EbpfInsn::call(BpfHelper::TracePrintk);
        let bytes = insn.encode();
        // opcode=0x85, imm=6 (TracePrintk helper number)
        assert_eq!(bytes, [0x85, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_builder() {
        let mut builder = EbpfBuilder::new();
        builder
            .push(EbpfInsn::mov64_imm(EbpfReg::R0, 0))
            .push(EbpfInsn::exit());

        let bytecode = builder.build();
        assert_eq!(bytecode.len(), 16); // 2 instructions * 8 bytes
    }
}
