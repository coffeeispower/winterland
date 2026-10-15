zig := 'zig'

test file:
    cd winterland_shovel && {{zig}} test src/{{file}}.zig
alias t := test
build:
    {{zig}} build --build-file winterland_shovel/build.zig
