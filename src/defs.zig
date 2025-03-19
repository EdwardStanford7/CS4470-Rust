const std = @import("std");
pub const tipe = enum(u8) {
    start,
    accept,
    invalid,
    ignore, // Ignore state - resets to start without committing token
    //--keywords:
    ASSERT,
    ARRAY,
    BOOL,
    ELSE,
    FALSE,
    FN,
    FLOAT,
    INT,
    IMAGE,
    IF,
    LET,
    PRINT,
    RETURN,
    READ,
    STRUCT,
    SHOW,
    SUM,
    TO,
    TIME,
    TRUE,
    THEN,
    VOID,
    WRITE,
    //---
    VARIABLE,
    COLON,
    RPAREN,
    LPAREN,
    RCURLY,
    RSQUARE,
    LSQUARE,
    LCURLY,
    NEWLINE,
    INTVAL,
    FLOATVAL,
    OP,
    COMMA,
    EQUALS,
    STRING,
    DOT,
    END_OF_FILE,
};

pub const keywords = [_]tipe{
    .ASSERT,
    .ARRAY,
    .BOOL,
    .ELSE,
    .FALSE,
    .FN,
    .FLOAT,
    .INT,
    .IMAGE,
    .IF,
    .LET,
    .PRINT,
    .RETURN,
    .READ,
    .STRUCT,
    .SHOW,
    .SUM,
    .TO,
    .TIME,
    .TRUE,
    .THEN,
    .VOID,
    .WRITE,
};

pub const offset_size = u32;
pub const value = [2]offset_size;

pub const tabs_needed = 125; // The number of states in the state machine
pub const biggest_char = 256;
pub const state_type = u8;
pub const trie_node: type = [biggest_char]state_type;

// Define special state IDs
pub const error_state: state_type = 0;
pub const start_state: state_type = 1;
pub const accept_state: state_type = 2;
pub const ignore_state: state_type = 3; // The ignore state
pub const finish_state: state_type = 4;

pub const double_ops = [_][2]u8{
    .{ '|', '|' },
    .{ '&', '&' },
    .{ '=', '=' },
    .{ '!', '=' },
    .{ '>', '=' },
    .{ '<', '=' },
};

// Single operators
pub const single_ops = [_]struct { char: u8, tipe: tipe }{
    .{ .char = '(', .tipe = .LPAREN },
    .{ .char = ')', .tipe = .RPAREN },
    .{ .char = '{', .tipe = .LCURLY },
    .{ .char = '}', .tipe = .RCURLY },
    .{ .char = '[', .tipe = .LSQUARE },
    .{ .char = ']', .tipe = .RSQUARE },
    .{ .char = ':', .tipe = .COLON },
    .{ .char = ',', .tipe = .COMMA },
    .{ .char = '+', .tipe = .OP },
    .{ .char = '-', .tipe = .OP },
    .{ .char = '*', .tipe = .OP },
    .{ .char = '%', .tipe = .OP },
    .{ .char = '!', .tipe = .OP },
    .{ .char = '<', .tipe = .OP },
    .{ .char = '>', .tipe = .OP },
    .{ .char = '=', .tipe = .EQUALS },
};

pub const illegal_table: [biggest_char]bool = def: {
    var table: [biggest_char]bool = [1]bool{true} ** biggest_char;
    for (0x20..0x7E + 1) |index| {
        table[index] = false;
    }
    table[0x0A] = false;
    break :def table;
};

pub const illegal_string: [biggest_char]bool = def: {
    var table: [biggest_char]bool = [1]bool{true} ** biggest_char;
    for (0x20..0x7E + 1) |index| {
        table[index] = false;
    }
    break :def table;
};
