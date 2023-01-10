use std::io::{Read, Seek};

#[derive(Debug)]
pub enum Error {
    /// Failed to read the file
    ReadFile(std::io::Error),

    ReadFailure(std::io::Error),
    SeekFailure(std::io::Error),

    /// Elf magic number was wrong
    InvalidElfMagic,

    InvalidBitness,
    InvalidEndianness,
    InvalidOs(u8),
    InvalidElfType,
    InvalidMachine,
}


type Result<Res> = std::result::Result<Res, Error>;


#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Flags(u32);

impl Flags {
    /// Is the segment readable
    pub fn r(self) -> bool {
        self.0 & 0x4 != 0
    }

    /// Is the segment writable
    pub fn w(self) -> bool {
        self.0 & 0x2 != 0
    }

    /// Is the segment executable
    pub fn x(self) -> bool {
        self.0 & 0x1 != 0
    }
}

impl std::fmt::Debug for Flags {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut flags = String::new();
        if self.r() {
            flags += "R";
        } else {
            flags += "-";
        }
        if self.w() {
            flags += "W";
        } else {
            flags += "-";
        }
        if self.x() {
            flags += "X";
        } else {
            flags += "-";
        }
        write!(fmt, "{flags}")
    }
}


/// A segment in an ELF file
///
#[derive(Debug, Clone)]
pub struct Segment {
    /// Offset in file
    pub file_offset: u32,

    /// Size in file
    pub file_size: u32,

    /// Address to load at
    pub load_address: u32,

    /// Size in memory
    pub size: u32,

    /// Flags
    pub flags: Flags,

    /// Data in segment
    ///
    /// This is only the data in the file, length is file_size.
    pub data: Box<[u8]>,
}

#[derive(Debug)]
pub struct Elf {
    /// Entry point for the program
    pub entry: u32,

    /// Loadable segments
    pub load_segments: Vec<Segment>,
}

impl Elf {
    /// Read a file, verify it is a linux ELF exe and find the load segments.
    ///
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path);
        let mut file = file.map_err(Error::ReadFile)?;

        // the elf program header is 52 bytes on a 32 bit system
        let mut buf = [0u8; 52];
        file.read_exact(&mut buf[..]).map_err(Error::ReadFailure)?;

        // check the ELF magic number at the start of the file
        if buf[0..4] != [0x7f, 0x45, 0x4c, 0x46] {
            return Err(Error::InvalidElfMagic);
        }

        // check that it is a 32 bit executable
        if buf[4] != 1 {
            return Err(Error::InvalidBitness);
        }

        // check that it is little endian code
        if buf[5] != 1 {
            return Err(Error::InvalidEndianness);
        }

        // check that it is a system v executable (0)
        // TODO: should be linux? (0x03) or maybe not? abi is sysv?
        if buf[0x7] != 0 {
            return Err(Error::InvalidOs(buf[0]));
        }

        // check file type, should be a static exe ET_EXEC
        if buf[0x10..0x12] != [0x02, 0x0] {
            return Err(Error::InvalidElfType);
        }

        // check machine type, should be RISC-V
        if buf[0x12..0x14] != [0xf3, 0x0] {
            return Err(Error::InvalidMachine);
        }

        // get the entry point for the program
        let entry = u32::from_le_bytes(buf[0x18..0x1c].try_into().unwrap());


        // get the program header table offset
        let e_phoff = u32::from_le_bytes(buf[0x1c..0x20].try_into().unwrap()) as u64;

        // get the size of a program header entry
        let e_phentsize = u16::from_le_bytes(buf[0x2a..0x2c].try_into().unwrap()) as u64;

        // get the number of program header entries
        let e_phnum = u16::from_le_bytes(buf[0x2c..0x2e].try_into().unwrap()) as u64;

        // process all program header entries
        let mut load_segments = vec![];
        for entry_no in 0..e_phnum {
            // seek to the start of the entry
            file.seek(std::io::SeekFrom::Start(e_phoff + entry_no * e_phentsize))
                .map_err(Error::SeekFailure)?;

            // get the entry type
            // get the number of program header entries
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf[..]).map_err(Error::ReadFailure)?;
            let p_type = u32::from_le_bytes(buf);

            if p_type != 0x1 {
                // skip if type is not PT_LOAD
                continue;
            }

            // get the file offset for the load segment
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf[..]).map_err(Error::ReadFailure)?;
            let file_offset = u32::from_le_bytes(buf);

            // get the load address
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf[..]).map_err(Error::ReadFailure)?;
            let load_address = u32::from_le_bytes(buf);

            // skip p_paddr
            file.seek(std::io::SeekFrom::Current(4)).map_err(Error::SeekFailure)?;

            // get the file size for the load segment
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf[..]).map_err(Error::ReadFailure)?;
            let file_size = u32::from_le_bytes(buf);

            // get the memory size for the load segment
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf[..]).map_err(Error::ReadFailure)?;
            let size = u32::from_le_bytes(buf);

            // get the flags for the load segment
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf[..]).map_err(Error::ReadFailure)?;
            let flags = Flags(u32::from_le_bytes(buf));

            // read the data
            file.seek(std::io::SeekFrom::Start(file_offset as u64))
                .map_err(Error::SeekFailure)?;

            let mut data = Vec::with_capacity(file_size as usize);
            data.resize(size as usize, 0);
            file.read_exact(&mut data[..]).map_err(Error::ReadFailure)?;
            let data = data.into_boxed_slice();

            load_segments.push(Segment {
                file_offset,
                file_size,
                load_address,
                size,
                flags,
                data,
            });
        }

        Ok(Elf {
            entry,
            load_segments,
        })
    }
}

/*
$ ../riscv-rv32i/bin/riscv32-unknown-elf-readelf -lh --dynamic ../test/test
ELF Header:
  Magic:   7f 45 4c 46 01 01 01 00 00 00 00 00 00 00 00 00
  Class:                             ELF32
  Data:                              2's complement, little endian
  Version:                           1 (current)
  OS/ABI:                            UNIX - System V
  ABI Version:                       0
  Type:                              EXEC (Executable file)
  Machine:                           RISC-V
  Version:                           0x1
  Entry point address:               0x100dc
  Start of program headers:          52 (bytes into file)
  Start of section headers:          23328 (bytes into file)
  Flags:                             0x0
  Size of this header:               52 (bytes)
  Size of program headers:           32 (bytes)
  Number of program headers:         3
  Size of section headers:           40 (bytes)
  Number of section headers:         21
  Section header string table index: 20

Program Headers:
  Type           Offset   VirtAddr   PhysAddr   FileSiz MemSiz  Flg Align
  RISCV_ATTRIBUT 0x003ee5 0x00000000 0x00000000 0x0001c 0x00000 R   0x1
  LOAD           0x000000 0x00010000 0x00010000 0x0366e 0x0366e R E 0x1000
  LOAD           0x003670 0x00014670 0x00014670 0x00854 0x008ac RW  0x1000

 Section to Segment mapping:
  Segment Sections...
   00     .riscv.attributes
   01     .text .rodata
   02     .eh_frame .init_array .fini_array .data .sdata .sbss .bss

There is no dynamic section in this file.
*/

