TEST = test.jpl
FLAGS = -O -whole-module-optimization -cross-module-optimization -lto=llvm-full
LINUXFLAGS =  -static-stdlib
DEBUGFLAGS = -g -sanitize=address,undefined
#main.swift lexer.swift -o main
SRCS = src/*.swift
MODE=-p
compile-fast:
	swiftc $(SRCS) -o main $(FLAGS)

compile:
	chmod +x setup-swift.sh
	sudo ./setup-swift.sh
	swiftc $(SRCS) -o main $(LINUXFLAGS)
co:
	@clear
	zig build-exe src/main.zig -OReleaseFast

dumpsrc:
	@clear
	sh printsrc.sh > outsrc.txt

compile-debug:
	swiftc $(SRCS) -o main $(DEBUGFLAGS)
compile-intended:
	swift build -c debug -v
run-fast:	
	./main $(TEST) -l
run-intended:
	swift run --skip-build 4470-Compiler -- $(TEST) -p
run:
	./main $(TEST) -l
norm-test:
	@clear
	zig build-exe src/main.zig -OReleaseFast && time ./main examples/t2.jpl -l > output.txt

check-ref:
	@clear
	zig build-exe src/main.zig -OReleaseFast
	./main examples/t2.jpl -l > output.txt
	(diff output.txt steoutput.expected > diff.txt) || code-insiders diff.txt

hyp:
	@clear
	zig build-exe src/main.zig -OReleaseFast
	hyperfine --warmup 3 './main examples/t2.jpl -l'

hyp-smol:
	@clear
	zig build-exe src/main.zig -OReleaseFast
	hyperfine --warmup 50 './main examples/t2.jpl -_'

hyp-smol-safe-smol-comp:
	@clear
	zig build-exe src/main.zig -OReleaseSafe
	hyperfine --warmup 10 './main examples/t2.jpl -_'
	zig build-exe src/main.zig -OReleaseSmall
	hyperfine --warmup 10 './main examples/t2.jpl -_'
	zig build-exe src/main.zig -OReleaseFast
	hyperfine --warmup 10 './main examples/t2.jpl -_'

norm-small:
	@clear
	zig build-exe src/main.zig -OReleaseFast && time ./main examples/gradient.jpl -l

ben:
	zig build-exe src/main.zig -OReleaseFast
	time ./main examples/t2.jpl -l &> /dev/null
	time ./main examples/t2.jpl -l > output.txt
	time ./main examples/t2.jpl -l &> /dev/null
	time ./main examples/t2.jpl -_ &> /dev/null


flamegraph:
	make compile-fast
	sudo dtrace -n 'profile-997 /execname == "main"/ { @[ustack(100)] = count(); }' -o out.stacks -c './main examples/t2.jpl -t' > /dev/null
	stackcollapse.pl out.stacks | flamegraph.pl --title "Flamegraph" > flamegraph.svg

zigraf:
	#!/bin/bash
	set -euo pipefail

	# Clean up previous artifacts
	echo "Cleaning up previous artifacts..."
	rm -f out.stacks flamegraphzig.svg

	# Build the Zig executable
	echo "Building the executable..."
	zig build-exe src/main.zig -OReleaseFast

	# Run DTrace profiling (redirecting extra output to /dev/null)
	echo "Profiling with DTrace..."
	sudo dtrace -n 'profile-997 /execname == "main"/ { @[ustack(100)] = count(); }' -o out.stacks -c './main examples/t2.jpl -t' > /dev/null

	# Generate the flamegraph from the stack data
	echo "Generating flamegraph..."
	stackcollapse.pl out.stacks | flamegraph.pl --title "Flamegraph" > flamegraphzig.svg

	echo "Flamegraph successfully generated as flamegraphzig.svg"


test2:
	@echo "Running grader tests..."
	make -C ./grader test-hw2 PART=all DIR=../

test3:
	@echo "Running grader tests..."
	make -C ./grader test-hw3 PART=all DIR=../

test4:
	@echo "Running grader tests..."
	make -C ./grader test-hw4 PART=all DIR=../

test5:
	@echo "Running grader tests..."
	make -C ./grader test-hw5 PART=all DIR=../

test6:
	@echo "Running grader tests..."
	make -C ./grader test-hw6 PART=all DIR=../

test7:
	@echo "Running grader tests..."
	make -C ./grader test-hw7 PART=all DIR=../
