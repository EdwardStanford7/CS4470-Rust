TEST = ./t.jpl
FLAGS ?=
BINARY = ./target/release/myjplc

compile: $(BINARY)

$(BINARY): src/*.rs
	cargo build --release
	
run: $(BINARY)
	$(BINARY) $(TEST) $(FLAGS)


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
	$(MAKE) -C ./grader test-hw15 PART=all

clean:
	cargo clean

.PHONY: compile run test run-all clean