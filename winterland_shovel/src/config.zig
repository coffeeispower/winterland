const std = @import("std");
const Allocator = std.mem.Allocator;

pub const Error = error{ FlakeNixNotFound, InvalidNixFile } || Allocator.Error;

pub const ShovelConfig = struct {
    flakePath: [:0]const u8,
    allocator: Allocator,
    const Self = @This();
    /// Creates a config for [flakeNixPath], or for `./flake.nix` when the
    /// path is omitted. The returned path is absolute and owned by `allocator`.
    pub fn new(
        flakeNixPath: ?[]const u8,
        allocator: Allocator,
        io: std.Io,
    ) Error!Self {
        const flakeNixPathOrDefault = flakeNixPath orelse "flake.nix";

        const flakePath = std.Io.Dir.cwd().realPathFileAlloc(
            io,
            flakeNixPathOrDefault,
            allocator,
        ) catch |err| switch (err) {
            error.OutOfMemory => return error.OutOfMemory,
            else => return error.FlakeNixNotFound,
        };
        errdefer allocator.free(flakePath);
        if (!std.mem.eql(u8, std.fs.path.extension(flakeNixPathOrDefault), ".nix")) {
            return error.InvalidNixFile;
        }
        return .{ .flakePath = flakePath, .allocator = allocator };
    }

    pub fn drop(self: *const Self) void {
        self.allocator.free(self.flakePath);
    }
};

test "returns FlakeNixNotFound when the file does not exist" {
    try std.testing.expectError(
        error.FlakeNixNotFound,
        ShovelConfig.new("does-not-exist/flake.nix", std.testing.allocator, std.testing.io),
    );
}

test "returns InvalidNixFile when the file is not a Nix file" {
    try std.testing.expectError(
        error.InvalidNixFile,
        ShovelConfig.new("src/config.zig", std.testing.allocator, std.testing.io),
    );
}

test "succeeds when the flake is valid" {
    const config = try ShovelConfig.new("ardos/flake.nix", std.testing.allocator, std.testing.io);
    defer config.drop();
    try std.testing.expectStringEndsWith(config.flakePath, "ardos/flake.nix");
}
