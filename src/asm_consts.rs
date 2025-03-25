/// Constants used in the assembly generator
pub mod asm_consts {
    // Section labels
    pub const DATA_SECTION: &str = "section .data\n";
    pub const TEXT_SECTION: &str = "\nsection .text\n";
    
    // Register names
    pub const RAX: &str = "rax";
    pub const RBP: &str = "rbp";
    pub const RSP: &str = "rsp";
    pub const RDI: &str = "rdi";
    pub const RSI: &str = "rsi";
    pub const RDX: &str = "rdx";
    pub const RCX: &str = "rcx";
    pub const R8: &str = "r8";
    pub const R9: &str = "r9";
    pub const R10: &str = "r10";
    pub const R12: &str = "r12";
    pub const R13: &str = "r13";
    pub const R14: &str = "r14";
    pub const R15: &str = "r15";
    pub const XMM0: &str = "xmm0";
    pub const XMM1: &str = "xmm1";
    
    // Argument registers in order (x86-64 calling convention)
    pub const ARG_REGISTERS: [&str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

    // Floating point instructions
    pub const XMM_MOVSD_FROM_STACK: &str = "\tmovsd xmm{}, [rsp]\n";
    pub const XMM_MOVSD_TO_STACK: &str = "\tmovsd [rsp], xmm{}\n";
    pub const XMM_ADD: &str = "\taddsd xmm0, xmm1\n";
    pub const XMM_SUB: &str = "\tsubsd xmm0, xmm1\n";
    pub const XMM_MUL: &str = "\tmulsd xmm0, xmm1\n";
    pub const XMM_DIV: &str = "\tdivsd xmm0, xmm1\n";
    pub const XMM_PXOR: &str = "\tpxor xmm0, xmm0\n";
    
    // Integer operations
    pub const INT_ADD: &str = "\tadd rax, r10\n";
    pub const INT_SUB: &str = "\tsub rax, r10\n";
    pub const INT_MUL: &str = "\timul rax, r10\n";
    pub const INT_DIV_PREP: &str = "\tcqo\n";
    pub const INT_DIV: &str = "\tidiv r10\n";
    pub const INT_MOV_REMAINDER: &str = "\tmov rax, rdx\n";
    pub const INT_NEG: &str = "\tneg rax\n";
    pub const INT_XOR_BOOL: &str = "\txor rax, 1\n";
    pub const INT_AND: &str = "\tand rax, r10\n";
    pub const INT_OR: &str = "\tor rax, r10\n";
    pub const INT_CMP: &str = "\tcmp rax, r10\n";
    pub const INT_TEST: &str = "\ttest rax, rax\n";
    pub const INT_SETE: &str = "\tsete al\n";
    pub const INT_SETNE: &str = "\tsetne al\n";
    pub const INT_SETL: &str = "\tsetl al\n";
    pub const INT_SETLE: &str = "\tsetle al\n";
    pub const INT_SETG: &str = "\tsetg al\n";
    pub const INT_SETGE: &str = "\tsetge al\n";
    pub const INT_AND_ONE: &str = "\tand rax, 1\n";
    
    // Function prologue and epilogue
    pub const FUNCTION_PROLOGUE_1: &str = "\tpush rbp\n";
    pub const FUNCTION_PROLOGUE_2: &str = "\tmov rbp, rsp\n";
    pub const FUNCTION_EPILOGUE_1: &str = "\tmov rsp, rbp\n";
    pub const FUNCTION_EPILOGUE_2: &str = "\tpop rbp\n";
    pub const FUNCTION_EPILOGUE_3: &str = "\tret\n";
    pub const FUNCTION_SAVE_REGS: &str = "\tpush r12\n\tpush r13\n\tpush r14\n\tpush r15\n";
    pub const FUNCTION_RESTORE_REGS: &str = "\tpop r15\n\tpop r14\n\tpop r13\n\tpop r12\n";
    
    // Error messages
    pub const DBZ: &str = "divide by zero";
    pub const MBZ: &str = "mod by zero";
    
    // Label prefixes
    pub const LABEL_ELSE: &str = "else";
    pub const LABEL_ENDIF: &str = "endif";
    pub const LABEL_ASSERT_PASS: &str = "assert_pass";
    pub const JUMP: &str = "jump";
    
    // Stack alignment
    pub const STACK_ALIGN_COMMENT: &str = "\tsub rsp, 8 ; Align stack\n";
    pub const STACK_UNALIGN_COMMENT: &str = "\tadd rsp, 8 ; Remove alignment\n";

    // Main function labels
    pub const MAIN_LABEL_1: &str = "jpl_main:\n";
    pub const MAIN_LABEL_2: &str = "_jpl_main:\n";
    
    // Import declarations
    pub const IMPORTS: &str = "\tglobal jpl_main\n\
\tglobal _jpl_main\n\
\textern _fail_assertion\n\
\textern _jpl_alloc\n\
\textern _get_time\n\
\textern _show\n\
\textern _print\n\
\textern _print_time\n\
\textern _read_image\n\
\textern _write_image\n\
\textern _fmod\n\
\textern _sqrt\n\
\textern _exp\n\
\textern _sin\n\
\textern _cos\n\
\textern _tan\n\
\textern _asin\n\
\textern _acos\n\
\textern _atan\n\
\textern _log\n\
\textern _pow\n\
\textern _atan2\n\
\textern _to_int\n\
\textern _to_float\n\
\n";

    // TODO comments
    pub const TODO_ARRAY_INDEX: &str = "// TODO: Implement array indexing\n";
    pub const TODO_ARRAY_LITERALS: &str = "// TODO: Implement array literals\n";

}