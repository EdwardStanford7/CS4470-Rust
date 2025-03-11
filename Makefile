TEST=./t.jpl
COMPILE_MODE =-m lex

test:
	$(MAKE) -C ./grader test-hw2 PART=all

run:
	cargo run --release -- $(TEST) $(COMPILE_MODE)

time-all:
	time $(MAKE) run-all

run-all:
	$(MAKE) -C ./grader test-hw2 PART=all COMPILE_MODE=-l
	$(MAKE) -C ./grader test-hw3 PART=all COMPILE_MODE=-p
	$(MAKE) -C ./grader test-hw4 PART=all COMPILE_MODE=-p
	$(MAKE) -C ./grader test-hw5 PART=all COMPILE_MODE=-p
	$(MAKE) -C ./grader test-hw6 PART=all COMPILE_MODE=-t
	$(MAKE) -C ./grader test-hw7 PART=all COMPILE_MODE=-t
	$(MAKE) -C ./grader test-hw8 PART=all COMPILE_MODE=-i
	$(MAKE) -C ./grader test-hw9 PART=all COMPILE_MODE=-i

.PHONY: run time-all run-all