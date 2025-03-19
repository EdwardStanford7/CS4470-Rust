const std = @import("std");
const sm = @import("statemachine.zig");
const defs = @import("defs.zig");
const offset_size = defs.offset_size;
const state_machine = sm.state_machine;
const start_state = defs.start_state;
const error_state = defs.error_state;
const accept_state = defs.accept_state;
const ignore_state = defs.ignore_state;
const finish_state = defs.finish_state;
const value = defs.value;
const tipe = defs.tipe;
const state_type = defs.state_type;

pub var n: offset_size = 0;
pub var i: offset_size = 0;
pub var j: offset_size = 0;

pub fn lex(string: []u8, values: []value, types: []tipe) void {
    n = 0;
    i = 0;
    j = 0;
    var state: state_type = start_state;
    l: switch (state_machine.tabs[state][string[j]]) {
        error_state => {
            @branchHint(.cold);
            if (j == string.len - 1) break :l else @panic("death");
        },
        accept_state => {
            values[n] = .{ i, j };
            types[n] = state_machine.t[state];
            n = n + 1;
            i = j;
            continue :l state_machine.tabs[start_state][string[j]];
        },
        ignore_state => {
            i = j + 1;
            j = j + 1;
            continue :l state_machine.tabs[start_state][string[j]];
        },
        else => |last_state| {
            @branchHint(.likely);
            state = last_state;
            j = j + 1;
            continue :l state_machine.tabs[last_state][string[j]];
        },
    }
    if (i != j) {
        const final_state = state_machine.t[state];
        if (final_state == .invalid) @panic("death");
        values[n] = .{ i, j };
        types[n] = final_state;
        n = n + 1;
        i = j;
    }
    values[n] = .{ i, j };
    types[n] = .END_OF_FILE;
    n = n + 1;
}
