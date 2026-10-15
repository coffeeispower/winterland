flags := "-collection:shovel=../winterland_shovel"

run:
    odin run . {{flags}}
build:
    odin build . {{flags}}
check:
    odin check . {{flags}}
