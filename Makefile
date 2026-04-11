.PHONY: clean test check check-examples

clean:
	@echo "Running cleanup script..."
	@./clean.sh

test:
	@echo "Running cargo test..."
	@cargo test

check:
	@echo "Running cargo check..."
	@cargo check

check-examples:
	@echo "Running example checks..."
	@bash scripts/check_examples.sh
