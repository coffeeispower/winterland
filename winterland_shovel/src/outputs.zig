const std = @import("std");
const Allocator = std.mem.Allocator;
const format = @import("std").fmt.allocPrint;
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
    children: []*Node,

    /// A non-owning pointer to the parent. The containing `Tree` owns every
    /// node; parents never own children through this field.
    parent: ?*const Node,

/// Builds the output name traversing the tree upwards and building the name.
/// The caller gains ownership of the returned string and must free it later with `allocator.free(...)`
pub fn buildNodeName(self: *const @This(), allocator: Allocator) []const u8 {
    // It first calculates the length by calculating the sum of the length of the names of the node plus its ancestors
    // including the dots separating the names. 
    var outputPathLength: usize = 0;
    var currentNode: ?*const Node = self;
    while (currentNode) |node| {
        outputPathLength += 1+node.name.len;
        currentNode = node.parent;
    }
    outputPathLength -= 1;

    // Use the length to allocate a new string with the exact length we need
    var outputPath = try allocator.alloc(u8, outputPathLength);
    errdefer allocator.free(outputPath);
    // Reset the currentNode to start iterating again
    currentNode = self;
    // Define an insert cursor to start inserting the strings from the end
    var insertIndex = outputPathLength;
    while (currentNode) |node| {
        // Setup the cursor to insert the current node's name
        insertIndex -= node.name.len;
        // Backup the cursor before inserting the dot
        const insertNamePos = insertIndex;
        // subtract one from the cursor if it needs to insert a dot
        if(insertIndex > 0) {
            insertIndex -= 1;
            outputPath[insertIndex] = '.';
        }
        // Copy the name of the current node to the span between the cursor position and cursor position + length of the node name.
        const slice = outputPath[(insertNamePos)..(insertNamePos+node.name.len)];
        @memcpy(slice, node.name);
        // Advance the current node to the parent node
        currentNode = node.parent;
    }
    return outputPath;
}

};

pub const Tree = struct {
    root: *Node,
    arena: std.heap.ArenaAllocator,

    pub fn drop(self: *Tree) void {
        self.arena.deinit();
    }

    pub fn clone(self: *const Tree) !Tree {
        var arena = std.heap.ArenaAllocator.init(self.arena.child_allocator);
        errdefer arena.deinit();

        return .{
            .root = try cloneNode(self.root, null, arena.allocator()),
            .arena = arena,
        };
    }

};

/// Prints the complete output tree for native debugging.
pub fn debugPrintOutputs(tree: *const Tree) void {
    std.debug.print("{s} [{s}]\n", .{ tree.root.name, @tagName(tree.root.kind) });

    var prefix = std.ArrayList(u8).empty;
    const allocator = @constCast(&tree.arena).allocator();
    defer prefix.deinit(allocator);
    debugPrintChildren(tree.root.children, &prefix, allocator);
}

fn debugPrintChildren(
    children: []*Node,
    prefix: *std.ArrayList(u8),
    allocator: Allocator,
) void {
    for (children, 0..) |child, index| {
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
    const base_allocator = shovel_config.allocator;
    var arena = std.heap.ArenaAllocator.init(base_allocator);
    errdefer arena.deinit();
    const allocator = arena.allocator();
    const flakePathArg = format(allocator, "path:{s}", .{ shovel_config.flakePath });
    defer allocator.free(flakePathArg);
    const result = std.process.run(base_allocator, io, .{
        .argv = &.{
            "nix",
            "flake",
            "show",
            "--json",
            "--all-systems",
            flakePathArg,
        },
        .stdout_limit = .limited(64 * 1024 * 1024),
        .stderr_limit = .limited(16 * 1024 * 1024),
    }) catch |err| switch (err) {
        error.OutOfMemory => return error.OutOfMemory,
        else => return error.NixCommandFailed,
    };
    defer base_allocator.free(result.stdout);
    defer base_allocator.free(result.stderr);

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
        .root = try nodeFromJson(allocator, null, "outputs", parsed.value),
        .arena = arena,
    };
}

fn nodeFromJson(
    allocator: Allocator,
    parent: ?*const Node,
    name: []const u8,
    value: std.json.Value,
) Allocator.Error!*Node {
    const owned_name = try allocator.dupe(u8, name);
    const node = allocator.create(Node) catch |err| {
        allocator.free(owned_name);
        return err;
    };
    node.* = .{
        .name = owned_name,
        .kind = undefined,
        .value = null,
        .children = &.{},
        .parent = parent,
    };

    switch (value) {
        .object => |object| {
            node.kind = objectKind(object);
            try appendObjectChildren(allocator, node, object.iterator());
        },
        .array => |array| {
            node.kind = .array;
            try appendArrayChildren(allocator, node, array.items);
        },
        .null => try setScalarValue(allocator, node, .null, "null"),
        .bool => |boolean| try setScalarValue(allocator, node, .boolean, if (boolean) "true" else "false"),
        .integer => |integer| try setFormattedScalarValue(allocator, node, .integer, "{d}", .{integer}),
        .float => |float| try setFormattedScalarValue(allocator, node, .float, "{d}", .{float}),
        .number_string => |number| try setScalarValue(allocator, node, .number_string, number),
        .string => |string| try setScalarValue(allocator, node, .string, string),
    }

    return node;
}

fn objectKind(object: std.json.ObjectMap) Kind {
    if (object.contains("derivation")) return .derivation;

    const type_value = object.get("type") orelse return .object;
    return switch (type_value) {
        .string => |type_name| if (std.mem.eql(u8, type_name, "derivation")) .derivation else .object,
        else => .object,
    };
}

fn appendObjectChildren(
    allocator: Allocator,
    node: *Node,
    iterator: anytype,
) Allocator.Error!void {
    var children = std.ArrayList(*Node).empty;
    errdefer {
        children.deinit(allocator);
    }

    var iter = iterator;
    while (iter.next()) |entry| {
        const child = try nodeFromJson(allocator, node, entry.key_ptr.*, entry.value_ptr.*);
        children.append(allocator, child) catch |err| {
            return err;
        };
    }

    node.children = try children.toOwnedSlice(allocator);
}

fn appendArrayChildren(
    allocator: Allocator,
    node: *Node,
    values: []const std.json.Value,
) Allocator.Error!void {
    var children = std.ArrayList(*Node).empty;
    errdefer {
        children.deinit(allocator);
    }

    for (values, 0..) |value, index| {
        const child_name = try std.fmt.allocPrint(allocator, "[{d}]", .{index});
        defer allocator.free(child_name);
        const child = try nodeFromJson(allocator, node, child_name, value);
        children.append(allocator, child) catch |err| {
            return err;
        };
    }

    node.children = try children.toOwnedSlice(allocator);
}

fn setScalarValue(allocator: Allocator, node: *Node, kind: Kind, value: []const u8) Allocator.Error!void {
    const owned_value = try allocator.dupe(u8, value);
    errdefer allocator.free(owned_value);
    node.kind = kind;
    node.value = owned_value;
}

fn setFormattedScalarValue(
    allocator: Allocator,
    node: *Node,
    kind: Kind,
    comptime format: []const u8,
    args: anytype,
) Allocator.Error!void {
    const owned_value = try std.fmt.allocPrint(allocator, format, args);
    errdefer allocator.free(owned_value);
    node.kind = kind;
    node.value = owned_value;
}

fn cloneNode(self: *const Node, parent: ?*const Node, allocator: Allocator) Allocator.Error!*Node {
    const node = try allocator.create(Node);
    const owned_name = allocator.dupe(u8, self.name) catch |err| {
        allocator.destroy(node);
        return err;
    };
    node.* = .{
        .name = owned_name,
        .kind = self.kind,
        .value = null,
        .children = &.{},
        .parent = parent,
    };
    if (self.value) |value| node.value = try allocator.dupe(u8, value);

    var children = std.ArrayList(*Node).empty;
    errdefer {
        children.deinit(allocator);
    }
    for (self.children) |child| {
        const cloned_child = try cloneNode(child, node, allocator);
        children.append(allocator, cloned_child) catch |err| {
            return err;
        };
    }
    node.children = try children.toOwnedSlice(allocator);
    return node;
}

test "converts JSON flake output into a tree" {
    var parsed = try std.json.parseFromSlice(
        std.json.Value,
        std.testing.allocator,
        "{\"packages\":{\"x86_64-linux\":{\"hello\":{\"derivation\":{\"name\":\"hello\"},\"forSystems\":[\"x86_64-linux\"],\"what\":\"package\"}}}}",
        .{},
    );
    defer parsed.deinit();

    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    var tree = Tree{
        .root = try nodeFromJson(arena.allocator(), null, "outputs", parsed.value),
        .arena = arena,
    };
    defer tree.drop();

    debugPrintOutputs(&tree);
    try std.testing.expectEqual(1, tree.root.children.len);
    try std.testing.expect(tree.root.parent == null);
    try std.testing.expectEqualStrings("packages", tree.root.children[0].name);
    try std.testing.expect(tree.root.children[0].parent.? == tree.root);
    try std.testing.expect(tree.root.children[0].children[0].parent.? == tree.root.children[0]);
    try std.testing.expectEqual(.derivation, tree.root.children[0].children[0].children[0].kind);
    try std.testing.expectEqualStrings("hello", tree.root.children[0].children[0].children[0].children[0].children[0].value.?);

    var cloned_tree = try tree.clone();
    defer cloned_tree.drop();
    try std.testing.expect(cloned_tree.root != tree.root);
    try std.testing.expect(cloned_tree.root.children[0].parent.? == cloned_tree.root);
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
    try std.testing.expect(containsKind(tree.root, .derivation));
    try std.testing.expect(containsDerivationName(tree.root, "ardos-rom.squashfs"));
}

fn containsKind(node: *const Node, kind: Kind) bool {
    if (node.kind == kind) return true;
    for (node.children) |child| {
        if (containsKind(child, kind)) return true;
    }
    return false;
}

fn containsDerivationName(node: *const Node, name: []const u8) bool {
    if (node.kind == .derivation) {
        for (node.children) |child| {
            if (!std.mem.eql(u8, child.name, "derivation")) continue;
            for (child.children) |field| {
                if (std.mem.eql(u8, field.name, "name") and
                    field.value != null and
                    std.mem.eql(u8, field.value.?, name)) return true;
            }
        }
    }

    for (node.children) |child| {
        if (containsDerivationName(child, name)) return true;
    }
    return false;
}
