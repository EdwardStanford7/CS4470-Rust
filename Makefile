TEST = ./t.jpl
COMPILE_MODE = -m parse
BINARY = target/release/myjplc

$(BINARY):	src/*.rs
	cargo build --release

run: $(BINARY)
	$(BINARY) $(TEST) $(COMPILE_MODE)

test:
	$(MAKE) -C ./grader test-hw5 PART=all

run-all: $(BINARY)
	$(MAKE) -C ./grader test-hw2 PART=all COMPILE_MODE=" -m lex"
	$(MAKE) -C ./grader test-hw3 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw4 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw5 PART=all COMPILE_MODE=" -m parse"
# $(MAKE) -C ./grader test-hw6 PART=all COMPILE_MODE=-t
# $(MAKE) -C ./grader test-hw7 PART=all COMPILE_MODE=-t
# $(MAKE) -C ./grader test-hw8 PART=all COMPILE_MODE=-i
# $(MAKE) -C ./grader test-hw9 PART=all COMPILE_MODE=-i

time-all:
	time $(MAKE) run-all

clean:
	cargo clean

.PHONY: run test build clean time-all run-all