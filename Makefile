TEST = ./t.jpl
COMPILE_MODE = -m typecheck
BINARY = target/release/myjplc
WHW = 10
WFD = ok
WNM = 048
WMO = assembly

instruct:
	@clear
	echo "input\n" && \
	cat grader/hw$(WHW)/$(WFD)/$(WNM).jpl && \
	echo "expected\n" && \
	cat grader/hw$(WHW)/$(WFD)/$(WNM).jpl.expected
	echo "actual\n" && \
	cargo run RUSTFLAGS=-Awarnings -- grader/hw$(WHW)/$(WFD)/$(WNM).jpl --mode $(WMO)



$(BINARY):	src/*.rs
	cargo build -q --release

run: $(BINARY)
	$(BINARY) $(TEST) $(COMPILE_MODE)

test:
	$(MAKE) -C ./grader test-hw10 PART=all

run-all: $(BINARY)
	$(MAKE) -C ./grader test-hw2 PART=all COMPILE_MODE=" -m lex"
	$(MAKE) -C ./grader test-hw3 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw4 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw5 PART=all COMPILE_MODE=" -m parse"
	$(MAKE) -C ./grader test-hw6 PART=all COMPILE_MODE=" -m typecheck"
	$(MAKE) -C ./grader test-hw7 PART=all COMPILE_MODE=" -m typecheck"
	$(MAKE) -C ./grader test-hw10 PART=all COMPILE_MODE=" -m assembly"

run-10:
	$(MAKE) -C ./grader test-hw10 PART=all COMPILE_MODE=" -m assembly"


time-all:
	time $(MAKE) run-all

clean:
	cargo -q clean

.PHONY: run test build clean time-all run-all