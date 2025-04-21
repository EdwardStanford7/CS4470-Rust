TEST = ./t.jpl
FLAGS ?=
BINARY = ./target/release/myjplc

compile: $(BINARY)

$(BINARY): src/*.rs
	cargo build --release

FO ?= grader
SU ?= hw12/sum
FI ?= 001.jpl

diff-one:
	@clear
	cargo run -- grader/$(FO)/$(SU)/$(FI).jpl -s -O1 > my.txt
	grader/jplc grader/$(FO)/$(SU)/$(FI).jpl -s -O1 > ref.txt
	code --diff my.txt ref.txt

run: $(BINARY)
	$(BINARY) $(TEST) $(FLAGS)

test:
	@clear
	$(MAKE) -C ./grader test-hw14 PART=all > output.txt || code output.txt

teste:
	@clear
	./grader/jplc examples/test.jpl -s > ref.txt && cargo run -- examples/test.jpl -s > my.txt

testi:
	@clear
	$(MAKE) -C ./grader test-hw12 PART=all > output.txt || open output.txt

run-all: $(BINARY)
	$(MAKE) -C ./grader test-hw2 PART=all
	$(MAKE) -C ./grader test-hw3 PART=all
	$(MAKE) -C ./grader test-hw4 PART=all
	$(MAKE) -C ./grader test-hw5 PART=all
	$(MAKE) -C ./grader test-hw6 PART=all
	$(MAKE) -C ./grader test-hw7 PART=all
	$(MAKE) -C ./grader test-hw10 PART=all
	$(MAKE) -C ./grader test-hw11 PART=all
	$(MAKE) -C ./grader test-hw12 PART=all
	$(MAKE) -C ./grader test-hw13 PART=all
	$(MAKE) -C ./grader test-hw14 PART=all

clean:
	cargo clean

.PHONY: compile run test run-all clean