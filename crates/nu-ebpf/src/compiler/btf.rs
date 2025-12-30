//! BTF (BPF Type Format) generation
//!
//! BTF is a metadata format that describes types used in BPF programs.
//! It's required by modern loaders (libbpf, Aya) for map definitions.
//!
//! References:
//! - https://docs.kernel.org/bpf/btf.html
//! - https://docs.ebpf.io/concepts/btf/

/// BTF magic number (little-endian)
const BTF_MAGIC: u16 = 0xEB9F;

/// BTF version
const BTF_VERSION: u8 = 1;

/// BTF type kinds
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum BtfKind {
    Unknown = 0,
    Int = 1,
    Ptr = 2,
    Array = 3,
    Struct = 4,
    Union = 5,
    Enum = 6,
    Fwd = 7,
    Typedef = 8,
    Volatile = 9,
    Const = 10,
    Restrict = 11,
    Func = 12,
    FuncProto = 13,
    Var = 14,
    DataSec = 15,
    Float = 16,
    DeclTag = 17,
    TypeTag = 18,
    Enum64 = 19,
}

/// BTF variable linkage
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum BtfVarLinkage {
    Static = 0,
    GlobalAlloc = 1,
    GlobalExtern = 2,
}

/// BTF type builder
pub struct BtfBuilder {
    /// String section (null-terminated strings)
    strings: Vec<u8>,
    /// Type section (encoded btf_type structs)
    types: Vec<u8>,
    /// Next type ID (starts at 1, 0 is void)
    next_type_id: u32,
}

impl BtfBuilder {
    pub fn new() -> Self {
        let mut builder = Self {
            strings: Vec::new(),
            types: Vec::new(),
            next_type_id: 1,
        };
        // First byte of string section must be null (for empty strings)
        builder.strings.push(0);
        builder
    }

    /// Add a string to the string section, return its offset
    fn add_string(&mut self, s: &str) -> u32 {
        let offset = self.strings.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        self.strings.push(0); // null terminator
        offset
    }

    /// Encode the info field: vlen (bits 0-15), kind (bits 24-28), kind_flag (bit 31)
    fn encode_info(kind: BtfKind, vlen: u16, kind_flag: bool) -> u32 {
        let mut info = vlen as u32;
        info |= (kind as u32) << 24;
        if kind_flag {
            info |= 1 << 31;
        }
        info
    }

    /// Add an integer type, return its type ID
    pub fn add_int(&mut self, name: &str, size: u32, is_signed: bool) -> u32 {
        let name_off = self.add_string(name);
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        // btf_type header
        self.types.extend_from_slice(&name_off.to_le_bytes());
        self.types.extend_from_slice(&Self::encode_info(BtfKind::Int, 0, false).to_le_bytes());
        self.types.extend_from_slice(&size.to_le_bytes()); // size in bytes

        // BTF_INT encoding: bits 0-7 = nr_bits, bits 16-23 = offset, bits 24-27 = encoding
        // See: https://docs.kernel.org/bpf/btf.html
        let nr_bits = size * 8;
        let encoding = if is_signed { 1u32 } else { 0u32 };
        let int_data = nr_bits | (encoding << 24);
        self.types.extend_from_slice(&int_data.to_le_bytes());

        type_id
    }

    /// Add a pointer type, return its type ID
    pub fn add_ptr(&mut self, target_type_id: u32) -> u32 {
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        // btf_type header (name_off = 0 for pointers)
        self.types.extend_from_slice(&0u32.to_le_bytes()); // name_off
        self.types.extend_from_slice(&Self::encode_info(BtfKind::Ptr, 0, false).to_le_bytes());
        self.types.extend_from_slice(&target_type_id.to_le_bytes()); // type

        type_id
    }

    /// Add an array type, return its type ID
    ///
    /// Arrays in BTF have an element type, index type (usually u32), and number of elements.
    pub fn add_array(&mut self, elem_type: u32, index_type: u32, nelems: u32) -> u32 {
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        // btf_type header (name_off = 0 for arrays, size = 0)
        self.types.extend_from_slice(&0u32.to_le_bytes()); // name_off
        self.types.extend_from_slice(&Self::encode_info(BtfKind::Array, 0, false).to_le_bytes());
        self.types.extend_from_slice(&0u32.to_le_bytes()); // size (unused for arrays)

        // btf_array data
        self.types.extend_from_slice(&elem_type.to_le_bytes());
        self.types.extend_from_slice(&index_type.to_le_bytes());
        self.types.extend_from_slice(&nelems.to_le_bytes());

        type_id
    }

    /// Create a __uint(name, val) type pattern: PTR -> ARRAY[val] -> INT
    ///
    /// This matches the libbpf macro: #define __uint(name, val) int (*name)[val]
    /// Used for BTF-defined map attributes like type, key_size, value_size, etc.
    pub fn add_uint_type(&mut self, int_type: u32, value: u32) -> u32 {
        // Create ARRAY[value] of int
        let array_type = self.add_array(int_type, int_type, value);
        // Create PTR to the array
        self.add_ptr(array_type)
    }

    /// Add a struct type, return its type ID
    ///
    /// Members are: (name, type_id, size_in_bytes)
    pub fn add_struct(&mut self, name: &str, members: &[(&str, u32, u32)]) -> u32 {
        let name_off = self.add_string(name);
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        // Calculate total size from members
        let size: u32 = members.iter().map(|(_, _, sz)| sz).sum();

        // btf_type header
        self.types.extend_from_slice(&name_off.to_le_bytes());
        self.types.extend_from_slice(&Self::encode_info(BtfKind::Struct, members.len() as u16, false).to_le_bytes());
        self.types.extend_from_slice(&size.to_le_bytes());

        // Member entries
        let mut offset = 0u32;
        for (member_name, member_type, member_size) in members {
            let member_name_off = self.add_string(member_name);
            self.types.extend_from_slice(&member_name_off.to_le_bytes());
            self.types.extend_from_slice(&member_type.to_le_bytes());
            self.types.extend_from_slice(&(offset * 8).to_le_bytes()); // offset in bits
            offset += member_size;
        }

        type_id
    }

    /// Add an anonymous struct (no name) with pointer-sized members
    ///
    /// This is used for BTF-defined maps where all fields are pointers.
    /// Members are: (name, type_id) - all members are pointer-sized (8 bytes)
    pub fn add_btf_map_struct(&mut self, members: &[(&str, u32)]) -> u32 {
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        // Size = number of members * 8 (pointer size)
        let size = (members.len() * 8) as u32;

        // btf_type header (name_off = 0 for anonymous struct)
        self.types.extend_from_slice(&0u32.to_le_bytes()); // name_off = 0 (anonymous)
        self.types.extend_from_slice(&Self::encode_info(BtfKind::Struct, members.len() as u16, false).to_le_bytes());
        self.types.extend_from_slice(&size.to_le_bytes());

        // Member entries (each pointer is 8 bytes)
        for (idx, (member_name, member_type)) in members.iter().enumerate() {
            let member_name_off = self.add_string(member_name);
            self.types.extend_from_slice(&member_name_off.to_le_bytes());
            self.types.extend_from_slice(&member_type.to_le_bytes());
            let offset_bits = (idx * 8 * 8) as u32; // offset in bits
            self.types.extend_from_slice(&offset_bits.to_le_bytes());
        }

        type_id
    }

    /// Add a variable, return its type ID
    pub fn add_var(&mut self, name: &str, type_id: u32, linkage: BtfVarLinkage) -> u32 {
        let name_off = self.add_string(name);
        let var_type_id = self.next_type_id;
        self.next_type_id += 1;

        // btf_type header
        self.types.extend_from_slice(&name_off.to_le_bytes());
        self.types.extend_from_slice(&Self::encode_info(BtfKind::Var, 0, false).to_le_bytes());
        self.types.extend_from_slice(&type_id.to_le_bytes());

        // btf_var
        self.types.extend_from_slice(&(linkage as u32).to_le_bytes());

        var_type_id
    }

    /// Add a datasec (describes a section like .maps), return its type ID
    pub fn add_datasec(&mut self, name: &str, vars: &[(u32, u32, u32)]) -> u32 {
        let name_off = self.add_string(name);
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        // Calculate total size
        let size: u32 = vars.iter().map(|(_, _, sz)| sz).sum();

        // btf_type header
        self.types.extend_from_slice(&name_off.to_le_bytes());
        self.types.extend_from_slice(&Self::encode_info(BtfKind::DataSec, vars.len() as u16, false).to_le_bytes());
        self.types.extend_from_slice(&size.to_le_bytes());

        // btf_var_secinfo entries
        for (var_type_id, offset, size) in vars {
            self.types.extend_from_slice(&var_type_id.to_le_bytes());
            self.types.extend_from_slice(&offset.to_le_bytes());
            self.types.extend_from_slice(&size.to_le_bytes());
        }

        type_id
    }

    /// Build the complete BTF blob
    pub fn build(self) -> Vec<u8> {
        let hdr_len = 24u32; // Size of btf_header
        let type_off = 0u32;
        let type_len = self.types.len() as u32;
        let str_off = type_len;
        let str_len = self.strings.len() as u32;

        let mut btf = Vec::with_capacity(hdr_len as usize + type_len as usize + str_len as usize);

        // Header
        btf.extend_from_slice(&BTF_MAGIC.to_le_bytes());
        btf.push(BTF_VERSION);
        btf.push(0); // flags
        btf.extend_from_slice(&hdr_len.to_le_bytes());
        btf.extend_from_slice(&type_off.to_le_bytes());
        btf.extend_from_slice(&type_len.to_le_bytes());
        btf.extend_from_slice(&str_off.to_le_bytes());
        btf.extend_from_slice(&str_len.to_le_bytes());

        // Type section
        btf.extend_from_slice(&self.types);

        // String section
        btf.extend_from_slice(&self.strings);

        btf
    }
}

/// Generate BTF for a perf event array map
pub fn generate_perf_map_btf(map_name: &str) -> Vec<u8> {
    let mut btf = BtfBuilder::new();

    // Add basic types
    let u32_type = btf.add_int("__u32", 4, false);

    // For a perf event array, we define a struct with the map attributes
    // This matches what libbpf/Aya expects for BTF-defined maps
    let map_struct = btf.add_struct(map_name, &[
        ("type", u32_type, 4),
        ("key_size", u32_type, 4),
        ("value_size", u32_type, 4),
        ("max_entries", u32_type, 4),
    ]);

    // Add a variable for the map
    let map_var = btf.add_var(map_name, map_struct, BtfVarLinkage::GlobalAlloc);

    // Add datasec for .maps section
    btf.add_datasec(".maps", &[(map_var, 0, 16)]);

    btf.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btf_builder() {
        let btf = generate_perf_map_btf("events");

        // Check magic
        assert_eq!(&btf[0..2], &BTF_MAGIC.to_le_bytes());

        // Check version
        assert_eq!(btf[2], BTF_VERSION);

        // Should have reasonable size
        assert!(btf.len() > 24); // At least header size
    }

    #[test]
    fn test_btf_int() {
        let mut btf = BtfBuilder::new();
        let type_id = btf.add_int("int", 4, true);
        assert_eq!(type_id, 1); // First type after void

        let data = btf.build();
        assert!(!data.is_empty());
    }
}
