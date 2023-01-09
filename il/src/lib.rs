use std::collections::BTreeMap;


/// A function id
///
/// Function ids must be globally unique.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FunctionId(pub u32);


/// A block id
///
/// Block ids must be unique in a function.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockId(pub u32);

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b{}", self.0)
    }
}


/// A name
///
/// Names are unique in a function.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name(u32);

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}


/// Possible binary operations
///
#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    LessThan,
    Eq,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BinOp::*;
        let op = match self {
            Add => "+",
            Sub => "-",
            LessThan => "<",
            Eq => "==",
        };
        write!(f, "{op}")
    }
}


/// A [`Name`] or a [`u32`]
///
#[derive(Debug, Clone)]
pub enum NameOrVal {
    Name(Name),
    Val(u32),
}

impl std::fmt::Display for NameOrVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameOrVal::Name(name) => write!(f, "{name}"),
            NameOrVal::Val(val) => write!(f, "{val:#08x}"),
        }
    }
}


/// A basic block of instructions.
///
/// A stream of instructions with optionally named results.
///
/// Invariants:
///
/// - instruction stream cannot be empty.
/// - any phi nodes must come first (TODO: incorporate into Block definition?)
/// - Ret, Branch and Cond must only appear as the last instruction
/// - the block must end in a Ret, Branch or Cond instruction
///
#[derive(Debug, Clone)]
pub struct Block {
    /// Instruction stream
    pub instructions: Vec<(Option<Name>, Instr)>,
}


/// Supported instructions in the IL
#[derive(Debug, Clone)]
pub enum Instr {
    /// Phi instruction
    ///
    /// Appears that the start of a block.
    Phi {
        /// Mapping from block we came from to value
        assignments: BTreeMap<BlockId, NameOrVal>,
    },

    /// Unconditional jump
    Branch {
        /// Block to jump to
        dest: BlockId,
    },

    /// A conditional jump
    Cond {
        /// Condition
        val: NameOrVal,

        /// Block to jump to if condition is true
        true_dest: BlockId,

        /// Block to jump to if condition is false
        false_dest: BlockId,
    },

    /// A function call
    Call {
        /// Function to call
        function: FunctionId,

        /// Arguments
        args: Vec<NameOrVal>,
    },

    /// Return from function
    Return {
        /// Return value, zero or more
        vals: Vec<NameOrVal>,
    },

    /// A literal value
    Literal {
        val: u32,
    },

    /// A binary operation
    BinOp {
        /// Left hand value
        a: NameOrVal,
        /// Operation
        op: BinOp,
        /// Right hand value
        b: NameOrVal,
    },

    /// Read a memory address
    ReadMem {
        /// Address to read
        addr: NameOrVal,
    },

    /// Write a memory address
    WriteMem {
        /// Address to write
        addr: NameOrVal,
        /// Value to write
        val: NameOrVal,
    },
}


/// A function is a collection of basic blocks
///
pub struct Function {
    /// Entry block id
    ///
    /// This block must exist in the block map.
    pub entry: BlockId,

    /// Number of arguments
    pub nargs: u32,

    /// Instruction blocks
    pub blocks: BTreeMap<BlockId, Block>,
}


/// Pretty print a function
///
pub fn print_function(name: &str, fun: &Function) {
    let mut out = String::from(name);
    out += "(";
    for argno in 1..=fun.nargs {
        out += &format!("{}, ", Name(argno));

    }
    out += ")";
    println!("{out}");
    for (&blockid, block) in &fun.blocks {
        println!("{}:", blockid);
        for (var, instr) in &block.instructions {
            let mut out = String::from("    ");

            if let Some(name) = var {
                out += &format!("{} = ", name);
            }

            use Instr::*;
            match instr {
                Phi { assignments } => {
                    out += &"phi ";
                    for (block, var) in assignments {
                        out += &format!("[ {}: {} ] ", block, var);
                    }
                },
                Cond { val, true_dest, false_dest } => {
                    out += &format!("cond {val}, {true_dest}, {false_dest}");
                },
                Branch { dest } => {
                    out += &format!("branch {dest}");
                },
                Call { function, args } => {
                    out += &format!("call {}(", function.0);
                    for var in args {
                        out += &format!("{var}, ");
                    }
                    out += ")";
                },
                Return { vals } => {
                    out += "ret ";
                    for var in vals {
                        out += &format!("{var}, ");
                    }
                }
                Literal { val } => {
                    out += &format!("{val:#08x}");
                },
                BinOp { a, op, b } => {
                    out += &format!("{a} {op} {b}");
                },
                ReadMem { addr } => {
                    out += &format!("read *{addr}");
                },
                WriteMem { addr, val } => {
                    out += &format!("write *{addr} {val}");
                },
            };

            println!("{out}");
        }
    }
}


/// [`BlockId`] generator
///
pub struct BlockGen {
    current: u32,
}

impl BlockGen {
    /// Create a new [`BlockId`] generator
    pub fn new() -> Self {
        BlockGen {
            current: 0,
        }
    }

    /// Get the next free [`BlockId`]
    pub fn next(&mut self) -> BlockId {
        let ret = BlockId(self.current);
        self.current += 1;
        ret
    }
}


/// [`Name`] generator
pub struct NameGen {
    current: u32,
}

impl NameGen {
    /// Create a new [`name`] generator
    pub fn new() -> Self {
        NameGen {
            current: 1,
        }
    }

    /// Get the next available [`Name`]
    pub fn next(&mut self) -> Name {
        let ret = Name(self.current);
        self.current += 1;
        ret
    }
}


#[test]
fn test() {
    let mut bg = BlockGen::new();
    let mut ng = NameGen::new();

    let max: Function = {
        let a = ng.next();
        let b = ng.next();
        let cond = ng.next();
        let ret = ng.next();

        let b0 = bg.next();
        let bt = bg.next();
        let bf = bg.next();
        let be = bg.next();

        use NameOrVal::*;

        let blocks = BTreeMap::from([
            (b0, Block { instructions: vec![
                (Some(cond), Instr::BinOp { a: Name(a), op: BinOp::LessThan, b: Name(b) } ),
                (None,       Instr::Cond { val: Name(cond), true_dest: bt, false_dest: bf }),
            ]}),
            (bt, Block { instructions: vec![
                (None, Instr::Branch { dest: be }),
            ]}),
            (bf, Block { instructions: vec![
                (None, Instr::Branch { dest: be }),
            ]}),
            (be, Block { instructions: vec![
                (Some(ret), Instr::Phi { assignments: BTreeMap::from([(bt, Name(b)), (bf, Name(a))]) }),
                (None,      Instr::Return { vals: vec![Name(ret)] }),
            ]}),
        ]);

        Function {
            blocks,
            nargs: 2,
            entry: b0,
        }
    };

    print_function("max", &max);
    println!("");

    let mut bg = BlockGen::new();
    let mut ng = NameGen::new();


    let write10: Function = {
        let addr = ng.next();
        let val = ng.next();

        let init = ng.next();
        let count = ng.next();
        let one = ng.next();
        let new_count = ng.next();
        let zero = ng.next();
        let cond = ng.next();

        let b0 = bg.next();
        let bloop = bg.next();
        let bend = bg.next();

        use NameOrVal::*;

        let blocks = BTreeMap::from([
            (b0, Block { instructions: vec![
                (Some(init), Instr::Literal { val: 10 }),
                (None,       Instr::Branch { dest: bloop }),
            ]}),
            (bloop, Block { instructions: vec![
                (Some(count),     Instr::Phi { assignments: BTreeMap::from([(b0, Name(init)), (bloop, Name(new_count))]) }),
                (None,            Instr::WriteMem { addr: Name(addr), val: Name(val) }),
                (Some(one),       Instr::Literal { val: 1 }),
                (Some(new_count), Instr::BinOp { a: Name(count), op: BinOp::Sub, b: Name(one) }),
                (Some(zero),      Instr::Literal { val: 0 }),
                (Some(cond),      Instr::BinOp { a: Name(new_count), op: BinOp::Eq, b: Name(zero) }),
                (None,            Instr::Cond { val: Name(cond), true_dest: bend, false_dest: bloop }),
            ]}),
            (bend, Block { instructions: vec![
                (None, Instr::Return { vals: vec![] }),
            ]}),
        ]);

        Function {
            blocks,
            nargs: 2,
            entry: b0,
        }
    };

    print_function("write10", &write10);
    println!("");

    let mut bg = BlockGen::new();
    let mut ng = NameGen::new();


    let memcpy: Function = {
        let from = ng.next();
        let to = ng.next();
        let count = ng.next();

        let zero = ng.next();
        let cond1 = ng.next();

        let count1 = ng.next();
        let count2 = ng.next();
        let from1 = ng.next();
        let from2 = ng.next();
        let to1 = ng.next();
        let to2 = ng.next();
        let byte = ng.next();
        let one = ng.next();
        let cond2 = ng.next();

        let b0 = bg.next();
        let bloop = bg.next();
        let bend = bg.next();

        use NameOrVal::*;

        let blocks = BTreeMap::from([
            (b0, Block { instructions: vec![
                (Some(zero),  Instr::Literal { val: 0 }),
                (Some(cond1), Instr::BinOp { a: Name(count), op: BinOp::Eq, b: Name(zero) }),
                (None,        Instr::Cond { val: Name(cond1), true_dest: bloop, false_dest: bend }),
            ]}),
            (bloop, Block { instructions: vec![
                (Some(count1),Instr::Phi { assignments:
                    BTreeMap::from([(b0, Name(count)), (bloop, Name(count2))]) }),
                (Some(from1), Instr::Phi { assignments:
                    BTreeMap::from([(b0, Name(from)), (bloop, Name(from2))]) }),
                (Some(to1),   Instr::Phi { assignments:
                    BTreeMap::from([(b0, Name(to)), (bloop, Name(to2))]) }),

                (Some(byte),  Instr::ReadMem { addr: Name(from1) }),
                (None,        Instr::WriteMem { addr: Name(to1), val: Name(byte) }),

                (Some(one),   Instr::Literal { val: 1 }),

                (Some(from2), Instr::BinOp { a: Name(from1), op: BinOp::Add, b: Name(one) }),
                (Some(to2),   Instr::BinOp { a: Name(to1), op: BinOp::Add, b: Name(one) }),
                (Some(count2),Instr::BinOp { a: Name(count1), op: BinOp::Sub, b: Name(one) }),

                (Some(cond2), Instr::BinOp { a: Name(count2), op: BinOp::Eq, b: Name(zero) }),
                (None,        Instr::Cond { val: Name(cond2), true_dest: bloop, false_dest: bend }),
            ]}),
            (bend, Block { instructions: vec![
                (None, Instr::Return { vals: vec![] }),
            ]}),
        ]);

        Function {
            blocks,
            nargs: 3,
            entry: b0,
        }
    };

    print_function("memcpy", &memcpy);
    println!("");
}
