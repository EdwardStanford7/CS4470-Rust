TEST ?= sample.jpl
FLAGS ?=

.PHONY: run compile test
compile:
	@chmod +x jplc

run: compile
	@./jplc $(FLAGS) $(TEST)

test: compile
	@chmod +x test.sh
	@./test.sh
	@chmod +x test-wasmgc.sh
	@./test-wasmgc.sh
