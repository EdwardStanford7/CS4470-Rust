const std = @import("std");
const lex = @import("lexer.zig");
const lexprinter = @import("lexprinter.zig");
const defs = @import("defs.zig");
pub fn panic(reason: []const u8, _: ?*std.builtin.StackTrace, _: ?usize) noreturn {
    //TODO recreate line number
    printLex() catch unreachable;
    _ = outw.print("reason: {s}\n", .{reason}) catch unreachable;
    _ = outw.print("last n: {d}, i: {d}, j: {d}\n", .{ lex.n, lex.i, lex.j }) catch unreachable;
    _ = outw.write("Compilation failed\n") catch unreachable;
    std.process.exit(0);
}

var outw: std.fs.File.Writer = undefined;
var lex_types: []defs.tipe = undefined;
var lex_values: []defs.value = undefined;
var imp: []u8 = undefined;
var v1: []u8 = undefined;
var v2: []u8 = undefined;

pub fn main() !void {
    outw = std.io.getStdOut().writer();
    // var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    // _ = gpa.allocator();

    //flags TODO swap and error message
    if (std.os.argv.len < 3) {
        _ = outw.write("not enough arguments\n") catch unreachable;
        return;
    }
    v1 = std.mem.span(std.os.argv[1][0..]);
    v2 = std.mem.span(std.os.argv[2][0..]);
    if (v1[0] == '-') {
        const v3 = v1;
        v1 = v2;
        v2 = v3;
    } else if (v2[0] != '-') {
        _ = outw.write("did not find a flag\n") catch unreachable;
        return;
    }
    const imp0 = std.fs.cwd().readFileAlloc(std.heap.page_allocator, v1, 1000000000) catch unreachable;
    imp = std.heap.page_allocator.realloc(imp0, imp0.len + 1) catch unreachable;
    imp[imp0.len] = 255;

    // if (evilBitCheck(imp0)) @panic("death"); //slightly slower
    defer std.heap.page_allocator.free(imp);
    //------------

    //make lex constructs
    lex_values = std.heap.page_allocator.alloc(defs.value, imp.len) catch unreachable;
    lex_types = std.heap.page_allocator.alloc(defs.tipe, imp.len) catch unreachable;
    defer std.heap.page_allocator.free(lex_values);
    defer std.heap.page_allocator.free(lex_types);
    //-------------

    lex.lex(imp, lex_values, lex_types);

    //----
    try printLex();

    //---
    _ = outw.write("Compilation succeeded\n") catch unreachable;
}

fn evilBitCheck(string: []u8) bool {
    const leng = std.simd.suggestVectorLength(u8) orelse @sizeOf(usize);
    for (0..string.len / leng) |index| {
        const base = leng * index;
        const slice1 = string[base..];
        const buff: @Vector(leng, u8) = slice1[0..leng].*;
        if (@reduce(.Or, buff) >> 7 != 0) {
            return true;
        }
    }
    for (0..string.len % leng) |index| {
        if (string[index] >> 7 != 0) return true;
    }
    return false;
}

fn printLex() !void {
    if (v2[1] == 'l') {
        var allocsize: usize = imp.len;
        allocsize *= 16;
        const lex_out = std.heap.page_allocator.alloc(u8, allocsize) catch @panic("could not allocate for printing");
        defer std.heap.page_allocator.free(lex_out);
        const len = try lexprinter.lex_print(lex_out, lex_values, lex_types, imp);
        _ = try std.posix.write(1, lex_out[0..len]);
    }
}
