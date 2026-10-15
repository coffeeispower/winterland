const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const module = b.addModule("winterland_shovel", .{
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });

    const dynamic_lib = b.addLibrary(.{
        .name = "winterland_shovel",
        .root_module = module,
        .linkage = .dynamic,
    });
    b.installArtifact(dynamic_lib);
    b.installFile("include/winterland_shovel.h", "include/winterland_shovel.h");
}
