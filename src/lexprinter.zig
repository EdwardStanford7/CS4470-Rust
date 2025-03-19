const std = @import("std");
const lex = @import("lexer.zig");
const defs = @import("defs.zig");
const value = defs.value;
const tipe = defs.tipe;
const u8_buff_writer = std.io.FixedBufferStream([]u8).Writer;
var scratch: [1 << 26]u8 = std.mem.zeroes([1 << 26]u8);

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

pub fn lex_print(out: u8_buff_writer, values: []value, types: []tipe, imp: []u8) bool {
    var scratch_i: usize = 0;
    for (0..lex.n) |i| {
        std.mem.copyForwards(u8, scratch[scratch_i..], @tagName(types[i]));
        scratch_i += @tagName(types[i]).len;
        if (types[i] != .NEWLINE and types[i] != .END_OF_FILE) {
            scratch[scratch_i] = ' ';
            scratch[scratch_i + 1] = '\'';
            scratch_i += 2;
            if (types[i] == .STRING) {
                scratch_i += addStringToBuffer(imp[values[i][0]..values[i][1]], scratch[scratch_i..]);
            } else {
                std.mem.copyForwards(u8, scratch[scratch_i..], imp[values[i][0]..values[i][1]]);
                scratch_i += values[i][1] - values[i][0];
            }
            scratch[scratch_i] = '\'';
            scratch_i += 1;
        }
        scratch[scratch_i] = '\n';
        scratch_i += 1;
        const res = out.write(scratch[0..scratch_i]) catch return false;
        if (res != scratch_i) {
            return false;
        }
        scratch_i = 0;
    }
    return true;
}
