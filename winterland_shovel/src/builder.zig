const shovel = @import("./lib.zig");
const Allocator = @import("std").mem.Allocator;
const StringBuilder = @import("std").ArrayList(u8);
const format = @import("std").fmt.allocPrint;

derivationNode: *shovel.outputs.Node,
outputPath: []const u8,
flakePath: []const u8,
allocator: Allocator,
const Self = @This();
pub fn init(output: *const shovel.outputs.Node, config: *const shovel.config.ShovelConfig, allocator: Allocator) !Self {
    return Self {
        .derivationNode = output,
        .outputPath = output.buildNodeName(allocator),
        .flakePath = allocator.dupe(u8, config.flakePath),
        .allocator = allocator
    };
}

pub fn drop(self: *Self) void {
    self.allocator.free(self.outputPath);
    self.allocator.free(self.flakePath);
}


/// Returns the flake url to pass to nix commands to refer to the output configured in this builder.
/// Make sure to free the returned slice, with the same allocator you used to call this function, later.
pub fn getFlakeUrl(self: *const Self, allocator: Allocator) ![]const u8 {
    return try format(
        allocator,
        "path:{s}#{s}",
        .{ self.flakePath, self.outputPath },
    );
}