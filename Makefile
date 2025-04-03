TEST = ./t.jpl
COMPILE_MODE = -m assembly
BINARY = target/release/myjplc

$(BINARY):	src/*.rs
	cargo build --release

# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
compile:
	cargo build

run: $(BINARY)
	$(BINARY) $(TEST) $(COMPILE_MODE)

test:
	$(MAKE) -C ./grader test-hw11 PART=all

test-last:
	$(MAKE) -C ./grader test-hw10 PART=all

run-all: $(BINARY)
	$(MAKE) -C ./grader test-hw2 PART=all COMPILE_MODE=" -m lex"
	$(MAKE) -C ./grader test-hw3 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw4 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw5 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw6 PART=all COMPILE_MODE=" -m typecheck"
	$(MAKE) -C ./grader test-hw7 PART=all COMPILE_MODE=" -m typecheck"
	$(MAKE) -C ./grader test-hw10 PART=all COMPILE_MODE=" -m assembly"
	$(MAKE) -C ./grader test-hw11 PART=all COMPILE_MODE=" -m assembly"

time-all:
	time $(MAKE) run-all

clean:
	cargo clean

.PHONY: run test test-last build clean time-all run-all