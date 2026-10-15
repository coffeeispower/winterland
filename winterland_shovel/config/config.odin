package config;

import "core:os"
import "core:strings"
import "base:runtime"
ShovelConfig :: struct {
	flakePath: string,
	allocator: runtime.Allocator
}

@(private) get_flake_path :: proc(
    maybe_path: Maybe(string),
    allocator: runtime.Allocator,
) -> (path: string, err: os.Error) {
    if value, ok := maybe_path.?; ok {
        path, alloc_err := strings.clone(value, allocator)
        if alloc_err != nil {
            return "", .Out_Of_Memory
        }
        return path, nil
    }

    return os.get_working_directory(allocator)
}
new :: proc(flakePath: Maybe(string), allocator := context.allocator) -> (config: ShovelConfig, err: os.Error) {
	return ShovelConfig {
		flakePath = get_flake_path(flakePath, allocator) or_return
	}, nil
}

drop :: proc(self: ^ShovelConfig) {
	delete(self.flakePath, self.allocator);
}

verify :: proc(self: ^ShovelConfig) -> Maybe(os.Error) {
	os.exists(strings.concatenate([?]string{self.flakePath, os.Path_Separator, "flake.nix"}, context.temp_allocator));

}
