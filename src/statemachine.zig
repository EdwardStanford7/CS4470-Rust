const lex = @import("lexer.zig");
const defs = @import("defs.zig");
const std = @import("std");
const keywords = defs.keywords;
const tabs_needed = defs.tabs_needed;
const biggest_char = defs.biggest_char;
const trie_node = defs.trie_node;
const error_state = defs.error_state;
const accept_state = defs.accept_state;
const start_state = defs.start_state;
const ignore_state = defs.ignore_state;
const single_ops = defs.single_ops;
const double_ops = defs.double_ops;
const illegal_table = defs.illegal_table;
const illegal_string = defs.illegal_string;
const tipe = defs.tipe;
const state_type = defs.state_type;

const StateMachine = struct {
    tabs: [tabs_needed]trie_node,
    t: [tabs_needed]tipe,
    tabs_alloc: state_type,

    fn init() StateMachine {
        var sm: StateMachine = undefined;
        for (0..tabs_needed) |index| {
            sm.tabs[index] = [1]state_type{error_state} ** biggest_char;
            sm.t[index] = .invalid;
        }
        sm.t[error_state] = .invalid;
        sm.t[accept_state] = .accept;
        sm.t[start_state] = .start;
        sm.t[ignore_state] = .ignore;
        sm.tabs_alloc = ignore_state + 1; // Start after the predefined states

        // Set up basic transitions
        setStateLoopAll(&sm, error_state); // Error state loops back to itself
        setStateTransitionAll(&sm, accept_state, start_state); // Accept -> Start
        setStateTransitionAll(&sm, ignore_state, start_state); // Ignore -> Start

        return sm;
    }

    fn allocRestricted(sm: *StateMachine, t: tipe) state_type {
        const state = sm.tabs_alloc;
        sm.tabs_alloc += 1;
        sm.t[state] = t;
        return state;
    }

    fn allocPermissive(sm: *StateMachine, t: tipe) state_type {
        const state = sm.tabs_alloc;
        sm.tabs_alloc += 1;
        sm.t[state] = t;
        for (0..biggest_char) |index| {
            if (illegal_table[index]) {
                sm.addTransition(state, error_state, index);
            } else {
                sm.addTransition(state, accept_state, index);
            }
        }
        return state;
    }

    fn allocVariable(sm: *StateMachine, var_state: state_type) state_type {
        const state = sm.tabs_alloc;
        sm.tabs_alloc += 1;
        sm.t[state] = .VARIABLE;
        for (0..biggest_char) |index| {
            if (illegal_table[index]) {
                sm.addTransition(state, error_state, index);
            } else {
                sm.addTransition(state, accept_state, index);
            }
        }
        sm.addTransition(state, var_state, '_');
        sm.addTransitionRange(state, var_state, 'a', 'z');
        sm.addTransitionRange(state, var_state, 'A', 'Z');
        sm.addTransitionRange(state, var_state, '0', '9');
        return state;
    }

    // Add a transition from one state to another for a specific character
    fn addTransition(sm: *StateMachine, from: state_type, to: state_type, ch: state_type) void {
        sm.tabs[from][ch] = to;
    }

    fn addTransitionLegal(sm: *StateMachine, from: state_type, to: state_type) void {
        for (0..biggest_char) |ch| {
            if (illegal_table[ch]) continue;
            sm.tabs[from][@intCast(ch)] = to;
        }
    }

    // Add transitions for all characters in a range
    fn addTransitionRange(sm: *StateMachine, from: state_type, to: state_type, start: state_type, end: state_type) void {
        for (start..end + 1) |ch| {
            sm.tabs[from][@intCast(ch)] = to;
        }
    }

    // Make a state loop back to itself for a specific character
    fn addSelfLoop(sm: *StateMachine, state: state_type, ch: state_type) void {
        sm.tabs[state][ch] = state;
    }

    // Make a state loop back to itself for a range of characters
    fn addSelfLoopRange(sm: *StateMachine, state: state_type, start: state_type, end: state_type) void {
        for (start..end + 1) |ch| {
            sm.tabs[state][@intCast(ch)] = state;
        }
    }

    fn addSelfLegal(sm: *StateMachine, state: state_type) void {
        for (0..biggest_char) |ch| {
            if (illegal_table[ch]) continue;
            sm.tabs[state][@intCast(ch)] = state;
        }
    }

    // Set all transitions from a state to go to another state
    fn setStateTransitionAll(sm: *StateMachine, from: state_type, to: state_type) void {
        for (0..biggest_char) |ch| {
            sm.tabs[from][@intCast(ch)] = to;
        }
    }

    // Make a state loop back to itself for all characters
    fn setStateLoopAll(sm: *StateMachine, state: state_type) void {
        for (0..biggest_char) |ch| {
            sm.tabs[state][@intCast(ch)] = state;
        }
    }

    fn addKeywords(sm: *StateMachine, var_state: state_type) void {
        for (keywords) |keyword| {
            var current = start_state;
            for (@tagName(keyword)) |ch| {
                const lower = ch | 0x20;
                current = sm.transitionNoLoopVar(current, lower, var_state);
            }
            sm.t[current] = keyword;
        }
    }

    fn transitionNoLoopVar(sm: *StateMachine, current: state_type, ch: state_type, var_state: state_type) state_type {
        const rejects = sm.tabs[current][ch] == error_state;
        const accepts = sm.tabs[current][ch] == accept_state;
        const loops = sm.tabs[current][ch] == current;
        const varloop = sm.tabs[current][ch] == var_state;
        if (rejects or accepts or loops or varloop) {
            const next = sm.allocVariable(var_state);
            _ = sm.addTransition(current, next, ch);
        }
        return sm.tabs[current][ch];
    }

    fn addOps(sm: *StateMachine) void {
        for (double_ops) |str| {
            const first = sm.addOpAllocFirst(str[0]);
            const second = sm.addOpAllocSecond(first, str[1]);
            sm.t[second] = .OP;
        }
    }

    fn addOpAllocFirst(sm: *StateMachine, ch: state_type) state_type {
        const loops = sm.tabs[start_state][ch] == start_state;
        const reject = sm.tabs[start_state][ch] == error_state;
        const accept = sm.tabs[start_state][ch] == accept_state;
        if (loops or reject or accept) {
            const next = sm.allocRestricted(.invalid);
            _ = sm.addTransition(start_state, next, ch);
        }
        return sm.tabs[start_state][ch];
    }

    fn addOpAllocSecond(sm: *StateMachine, current: state_type, ch: state_type) state_type {
        const loops = sm.tabs[current][ch] == current;
        const reject = sm.tabs[current][ch] == error_state;
        const accept = sm.tabs[current][ch] == accept_state;
        if (loops or reject or accept) {
            const next = sm.allocPermissive(.invalid);
            _ = sm.addTransition(current, next, ch);
        }
        return sm.tabs[current][ch];
    }

    // Add a token that consists of a single character
    fn addSingleCharTokens(sm: *StateMachine) void {
        for (single_ops) |op| {
            const state = sm.allocPermissive(op.tipe);
            _ = sm.addTransition(start_state, state, op.char);
        }
    }

    // Add comment handling (both line and block comments)
    fn addInvisHandling(sm: *StateMachine) void {
        const newline_state = sm.allocPermissive(.NEWLINE);
        const slash_state = sm.allocPermissive(.OP);
        const block_comment_state = sm.allocRestricted(.invalid);
        const line_comment_state = sm.allocRestricted(.invalid);
        const line_comment_leaving_state = sm.allocRestricted(.invalid);

        _ = sm.addTransition(start_state, ignore_state, ' ');
        _ = sm.addTransition(start_state, newline_state, '\n');
        _ = sm.addSelfLoop(newline_state, '\n');
        _ = sm.addSelfLoop(newline_state, ' ');
        _ = sm.addTransition(newline_state, slash_state, '/');
        _ = sm.addTransition(start_state, slash_state, '/');
        _ = sm.addTransition(slash_state, line_comment_state, '/');
        _ = sm.addSelfLegal(line_comment_state);
        _ = sm.addTransition(line_comment_state, newline_state, '\n');
        _ = sm.addTransition(slash_state, block_comment_state, '*');
        _ = sm.addSelfLegal(block_comment_state);
        _ = sm.addTransition(block_comment_state, line_comment_leaving_state, '*');
        _ = sm.addTransitionLegal(line_comment_leaving_state, block_comment_state);
        _ = sm.addTransition(line_comment_leaving_state, ignore_state, '/');
    }

    // Add string handling
    fn addStringHandling(sm: *StateMachine) void {
        const string_start = sm.allocRestricted(.invalid);
        _ = sm.addTransition(start_state, string_start, '"');

        const string_body = sm.allocRestricted(.invalid);

        // Add valid characters to string body
        for (0..biggest_char) |ch| {
            if (!illegal_string[@intCast(ch)] and ch != '"') {
                _ = sm.addTransition(string_start, string_body, @intCast(ch));
                _ = sm.addSelfLoop(string_body, @intCast(ch));
            }
        }

        // String end with quote
        const string_end = sm.allocPermissive(.STRING);
        _ = sm.addTransition(string_start, string_end, '"'); // Empty string
        _ = sm.addTransition(string_body, string_end, '"'); // String with content
    }

    // Add line continuation handling (backslash-newline)
    fn addLineContinuationHandling(sm: *StateMachine) void {
        const backslash_state = sm.allocRestricted(.invalid);
        _ = sm.addTransition(start_state, backslash_state, '\\');

        // Backslash followed by newline is ignored
        _ = sm.addTransition(backslash_state, ignore_state, '\n');

        // Any other character after backslash is an error
        for (0..biggest_char) |ch| {
            if (ch != '\n') {
                _ = sm.addTransition(backslash_state, error_state, @intCast(ch));
            }
        }
    }

    // Add number handling (integer and float)
    fn addNumberHandling(sm: *StateMachine) void {
        const int_state = sm.allocPermissive(.INTVAL);
        const dot_state = sm.allocPermissive(.DOT);
        const float_state = sm.allocPermissive(.FLOATVAL);

        // Integers
        for ('0'..'9' + 1) |digit| {
            _ = sm.addTransition(start_state, int_state, @intCast(digit));
            _ = sm.addSelfLoop(int_state, @intCast(digit));
        }

        // Dot
        _ = sm.addTransition(start_state, dot_state, '.');

        // Integer followed by dot becomes float
        _ = sm.addTransition(int_state, float_state, '.');

        // Dot followed by digit becomes float
        for ('0'..'9' + 1) |digit| {
            _ = sm.addTransition(dot_state, float_state, @intCast(digit));
            _ = sm.addSelfLoop(float_state, @intCast(digit));
        }
    }

    fn addVariableHandling(sm: *StateMachine) state_type {
        const var_state = sm.allocPermissive(.VARIABLE);
        _ = sm.addTransitionRange(start_state, var_state, 'a', 'z');
        _ = sm.addTransitionRange(start_state, var_state, 'A', 'Z');
        _ = sm.addTransition(start_state, var_state, '_');
        _ = sm.addSelfLoopRange(var_state, 'a', 'z');
        _ = sm.addSelfLoopRange(var_state, 'A', 'Z');
        _ = sm.addSelfLoopRange(var_state, '0', '9');
        _ = sm.addSelfLoop(var_state, '_');
        return var_state;
    }
};

// Create the state machine
pub const state_machine: StateMachine = def: {
    @setEvalBranchQuota(1000000);
    var sm = StateMachine.init();
    _ = sm.addInvisHandling();
    _ = sm.addSingleCharTokens();
    _ = sm.addOps();
    _ = sm.addKeywords(sm.addVariableHandling());
    _ = sm.addNumberHandling();
    _ = sm.addStringHandling();
    _ = sm.addLineContinuationHandling();
    break :def sm;
};

const expect = std.testing.expect;
test "fn" {
    const f_state = state_machine.tabs[start_state]['f'];
    const n_state = state_machine.tabs[f_state]['n'];
    const space_state = state_machine.tabs[n_state][' '];
    std.debug.print("\nf_state {d}", .{f_state});
    std.debug.print("\nn_state {d}", .{n_state});
    std.debug.print("\nspace_state {d}\n", .{space_state});
    try expect(state_machine.t[n_state] == .FN);
    try expect(space_state == accept_state);
}

test "newline" {
    const new_state = state_machine.tabs[start_state]['\n'];
    const new2_state = state_machine.tabs[new_state]['\n'];
    const other_state = state_machine.tabs[new2_state]['t'];
    try expect(state_machine.t[new2_state] == .NEWLINE);
    try expect(other_state == accept_state);
}
