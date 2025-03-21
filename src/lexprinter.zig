const std = @import("std");
const lex = @import("lexer.zig");
const defs = @import("defs.zig");
const value = defs.value;
const tipe = defs.tipe;

fn addStringToBuffer(src: []const u8, dst: []u8) usize {
    var i: u32 = 0;
    for (src) |c| {
        switch (c) {
            '\n' => {
                dst[i] = '\\';
                dst[i + 1] = 'n';
                i += 2;
            },
            else => {
                dst[i] = c;
                i += 1;
            },
        }
    }
    return i;
}

pub fn lex_print(outbuf: []u8, values: []value, types: []tipe, imp: []u8) !usize {
    var index: usize = 0;
    for (0..lex.n) |i| {
        std.mem.copyForwards(u8, outbuf[index..], @tagName(types[i]));
        index += @tagName(types[i]).len;
        if (types[i] != .NEWLINE and types[i] != .END_OF_FILE) {
            outbuf[index] = ' ';
            outbuf[index + 1] = '\'';
            index += 2;
            if (types[i] == .STRING) {
                index += addStringToBuffer(imp[values[i][0]..values[i][1]], outbuf[index..]);
            } else {
                std.mem.copyForwards(u8, outbuf[index..], imp[values[i][0]..values[i][1]]);
                index += values[i][1] - values[i][0];
            }
            outbuf[index] = '\'';
            index += 1;
        }
        outbuf[index] = '\n';
        index += 1;
    }
    return index;
}
