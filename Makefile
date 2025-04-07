TEST = ./t.jpl
COMPILE_MODE = -m assembly
BINARY = ./myjplc

$(BINARY):	src/*.rs
	cargo build --release
	cp target/release/myjplc .

debug: src/*.rs
	cargo build
	cp target/debug/myjplc .

FO ?= grader
SU ?= hw12/sum
FI ?= 001.jpl

diff-one:
	@clear
	cargo run -- grader/$(FO)/$(SU)/$(FI).jpl --mode assembly > my.txt
	grader/jplc grader/$(FO)/$(SU)/$(FI).jpl -s > ref.txt
	code-insiders --diff my.txt ref.txt

run: $(BINARY)
	$(BINARY) $(TEST) $(COMPILE_MODE)

test:
	@clear
	$(MAKE) -C ./grader test-hw12 PART=all > output.txt || code output.txt

testi:
	@clear
	$(MAKE) -C ./grader test-hw12 PART=all > output.txt || open output.txt

run-all: $(BINARY)
# $(MAKE) -C ./grader test-hw2 PART=all COMPILE_MODE=" -m lex"
# $(MAKE) -C ./grader test-hw3 PART=all COMPILE_MODE=" -m parse"
# $(MAKE) -C ./grader test-hw4 PART=all COMPILE_MODE=" -m parse"
# $(MAKE) -C ./grader test-hw5 PART=all COMPILE_MODE=" -m parse"
# $(MAKE) -C ./grader test-hw6 PART=all COMPILE_MODE=" -m typecheck"
# $(MAKE) -C ./grader test-hw7 PART=all COMPILE_MODE=" -m typecheck"
	$(MAKE) -C ./grader test-hw10 PART=all COMPILE_MODE=" -m assembly"
	$(MAKE) -C ./grader test-hw11 PART=all COMPILE_MODE=" -m assembly"
	$(MAKE) -C ./grader test-hw12 PART=all COMPILE_MODE=" -m assembly"

time-all:
	time $(MAKE) run-all

clean:
	cargo clean
	rm -f $(BINARY)

.PHONY: run test test-last build clean time-all run-all