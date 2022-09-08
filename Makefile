.PHONY: run
run:
	@cargo watch -x check -x test -x run

.PHONY: test
test:
	@cargo test
