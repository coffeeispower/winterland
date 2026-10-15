const std = @import("std");
const Allocator = std.mem.Allocator;

const config = @import("config.zig");

pub const Error = error{
    InvalidNixOutput,
    NixCommandFailed,
} || Allocator.Error;

/// The shape of a node in the flake output tree.
///
/// Objects and arrays have children. Scalar values have a textual value.
/// Every string in a tree is owned by the tree allocator.
pub const Kind = enum {
    object,
    derivation,
    array,
    null,
    boolean,
    integer,
    float,
    number_string,
    string,
};

pub const Node = struct {
    name: []const u8,
    kind: Kind,
    value: ?[]const u8,
    children: []Node,

    pub fn drop(self: *Node, allocator: Allocator) void {
        for (self.children) |*child| child.drop(allocator);
        allocator.free(self.children);
        allocator.free(self.name);
        if (self.value) |value| allocator.free(value);
    }
};

pub const Tree = struct {
    root: Node,
    allocator: Allocator,

    pub fn drop(self: *Tree) void {
        self.root.drop(self.allocator);
    }
};

/// Prints the complete output tree for native debugging.
pub fn debugPrintOutputs(tree: *const Tree) void {
    std.debug.print("{s} [{s}]\n", .{ tree.root.name, @tagName(tree.root.kind) });

    var prefix = std.ArrayList(u8).empty;
    defer prefix.deinit(tree.allocator);
    debugPrintChildren(tree.root.children, &prefix, tree.allocator);
}

fn debugPrintChildren(
    children: []const Node,
    prefix: *std.ArrayList(u8),
    allocator: Allocator,
) void {
    for (children, 0..) |*child, index| {
        const is_last = index == children.len - 1;
        const branch = if (is_last) "└── " else "├── ";

        std.debug.print(
            "{s}{s}{s} [{s}",
            .{ prefix.items, branch, child.name, @tagName(child.kind) },
        );
        if (child.value) |value| std.debug.print("]: {s}\n", .{value}) else std.debug.print("]\n", .{});

        const previous_length = prefix.items.len;
        prefix.appendSlice(allocator, if (is_last) "    " else "│   ") catch return;
        debugPrintChildren(child.children, prefix, allocator);
        prefix.shrinkRetainingCapacity(previous_length);
    }
}

/// Runs `nix flake show --json` and converts its result into an inspectable tree.
///
/// The caller owns the returned tree and must call `Tree.drop()`.
pub fn query_outputs(
    shovel_config: *const config.ShovelConfig,
    io: std.Io,
) Error!Tree {
    const allocator = shovel_config.allocator;
    const result = std.process.run(allocator, io, .{
        .argv = &.{
            "nix",
            "flake",
            "show",
            "--json",
            "--all-systems",
            shovel_config.flakePath,
        },
        .stdout_limit = .limited(64 * 1024 * 1024),
        .stderr_limit = .limited(16 * 1024 * 1024),
    }) catch |err| switch (err) {
        error.OutOfMemory => return error.OutOfMemory,
        else => return error.NixCommandFailed,
    };
    defer allocator.free(result.stdout);
    defer allocator.free(result.stderr);

    switch (result.term) {
        .exited => |code| if (code != 0) return error.NixCommandFailed,
        else => return error.NixCommandFailed,
    }

    var parsed = std.json.parseFromSlice(
        std.json.Value,
        allocator,
        result.stdout,
        .{},
    ) catch |err| switch (err) {
        error.OutOfMemory => return error.OutOfMemory,
        else => return error.InvalidNixOutput,
    };
    defer parsed.deinit();

    return .{
        .root = try nodeFromJson(allocator, "outputs", parsed.value),
        .allocator = allocator,
    };
}

fn nodeFromJson(allocator: Allocator, name: []const u8, value: std.json.Value) Allocator.Error!Node {
    const owned_name = try allocator.dupe(u8, name);
    errdefer allocator.free(owned_name);

    return switch (value) {
        .object => |object| nodeWithChildren(
            allocator,
            owned_name,
            objectKind(object),
            object.iterator(),
        ),
        .array => |array| nodeWithArrayChildren(allocator, owned_name, array.items),
        .null => scalarNode(allocator, owned_name, .null, "null"),
        .bool => |boolean| scalarNode(allocator, owned_name, .boolean, if (boolean) "true" else "false"),
        .integer => |integer| scalarAllocNode(allocator, owned_name, .integer, "{d}", .{integer}),
        .float => |float| scalarAllocNode(allocator, owned_name, .float, "{d}", .{float}),
        .number_string => |number| scalarNode(allocator, owned_name, .number_string, number),
        .string => |string| scalarNode(allocator, owned_name, .string, string),
    };
}

fn objectKind(object: std.json.ObjectMap) Kind {
    if (object.contains("derivation")) return .derivation;

    const type_value = object.get("type") orelse return .object;
    return switch (type_value) {
        .string => |type_name| if (std.mem.eql(u8, type_name, "derivation")) .derivation else .object,
        else => .object,
    };
}

fn nodeWithChildren(
    allocator: Allocator,
    name: []const u8,
    kind: Kind,
    iterator: anytype,
) Allocator.Error!Node {
    var children = std.ArrayList(Node).empty;
    errdefer {
        for (children.items) |*child| child.drop(allocator);
        children.deinit(allocator);
    }

    var iter = iterator;
    while (iter.next()) |entry| {
        const child = nodeFromJson(allocator, entry.key_ptr.*, entry.value_ptr.*) catch |err| {
            return err;
        };
        children.append(allocator, child) catch |err| {
            var owned_child = child;
            owned_child.drop(allocator);
            return err;
        };
    }

    const owned_children = try children.toOwnedSlice(allocator);
    return .{
        .name = name,
        .kind = kind,
        .value = null,
        .children = owned_children,
    };
}

fn nodeWithArrayChildren(allocator: Allocator, name: []const u8, values: []const std.json.Value) Allocator.Error!Node {
    var children = std.ArrayList(Node).empty;
    errdefer {
        for (children.items) |*child| child.drop(allocator);
        children.deinit(allocator);
    }

    for (values, 0..) |value, index| {
        const child_name = try std.fmt.allocPrint(allocator, "[{d}]", .{index});
        const child = nodeFromJson(allocator, child_name, value) catch |err| {
            allocator.free(child_name);
            return err;
        };
        allocator.free(child_name);
        children.append(allocator, child) catch |err| {
            var owned_child = child;
            owned_child.drop(allocator);
            return err;
        };
    }

    const owned_children = try children.toOwnedSlice(allocator);
    return .{
        .name = name,
        .kind = .array,
        .value = null,
        .children = owned_children,
    };
}

fn scalarNode(allocator: Allocator, name: []const u8, kind: Kind, value: []const u8) Allocator.Error!Node {
    const owned_value = try allocator.dupe(u8, value);
    errdefer allocator.free(owned_value);
    return .{
        .name = name,
        .kind = kind,
        .value = owned_value,
        .children = &.{},
    };
}

fn scalarAllocNode(
    allocator: Allocator,
    name: []const u8,
    kind: Kind,
    comptime format: []const u8,
    args: anytype,
) Allocator.Error!Node {
    const owned_value = try std.fmt.allocPrint(allocator, format, args);
    errdefer allocator.free(owned_value);
    return .{
        .name = name,
        .kind = kind,
        .value = owned_value,
        .children = &.{},
    };
}

test "converts JSON flake output into a tree" {
    var parsed = try std.json.parseFromSlice(
        std.json.Value,
        std.testing.allocator,
        "{\"packages\":{\"x86_64-linux\":{\"hello\":{\"derivation\":{\"name\":\"hello\"},\"forSystems\":[\"x86_64-linux\"],\"what\":\"package\"}}}}",
        .{},
    );
    defer parsed.deinit();

    var tree = Tree{
        .root = try nodeFromJson(std.testing.allocator, "outputs", parsed.value),
        .allocator = std.testing.allocator,
    };
    defer tree.drop();

    debugPrintOutputs(&tree);
    try std.testing.expectEqual(1, tree.root.children.len);
    try std.testing.expectEqualStrings("packages", tree.root.children[0].name);
    try std.testing.expectEqual(.derivation, tree.root.children[0].children[0].children[0].kind);
    try std.testing.expectEqualStrings("hello", tree.root.children[0].children[0].children[0].children[0].children[0].value.?);
}

test "ardos exposes derivation outputs" {
    var shovel_config = try config.ShovelConfig.new(
        "./ardos/flake.nix",
        std.testing.allocator,
        std.testing.io,
    );
    defer shovel_config.drop();
    var tree = try query_outputs(&shovel_config, std.testing.io);
    defer tree.drop();
    debugPrintOutputs(&tree);
    try std.testing.expect(containsKind(&tree.root, .derivation));
    try std.testing.expect(containsDerivationName(&tree.root, "ardos-rom.squashfs"));
}

fn containsKind(node: *const Node, kind: Kind) bool {
    if (node.kind == kind) return true;
    for (node.children) |*child| {
        if (containsKind(child, kind)) return true;
    }
    return false;
}

fn containsDerivationName(node: *const Node, name: []const u8) bool {
    if (node.kind == .derivation) {
        for (node.children) |*child| {
            if (!std.mem.eql(u8, child.name, "derivation")) continue;
            for (child.children) |*field| {
                if (std.mem.eql(u8, field.name, "name") and
                    field.value != null and
                    std.mem.eql(u8, field.value.?, name)) return true;
            }
        }
    }

    for (node.children) |*child| {
        if (containsDerivationName(child, name)) return true;
    }
    return false;
}
