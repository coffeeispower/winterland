package main
import "shovel:shovel"
import "core:fmt"
import "core:os"
import process "core:os"
import shovelConfig "shovel:config"

run :: proc() -> Maybe(os.Error) {
	config := shovelConfig.new(flakePath = "someFlake/") or_return;
	defer shovelConfig.drop(&config);
	return nil
}

main :: proc() {
	err := run();
	if err != nil {
		fmt.eprintln("Error:", err);
		process.exit(1);
	}
}
