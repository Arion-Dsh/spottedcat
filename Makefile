.PHONY: clean test check

clean:
	@echo "Running cleanup script..."
	@./clean.sh

test:
	@echo "Running cargo test..."
	@cargo test

check:
	@echo "Running cargo check..."
	@cargo check
