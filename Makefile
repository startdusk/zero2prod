.PHONY: run
run:
	@cargo watch -x check -x test -x run

.PHONY: test
test:
	@cargo test

SKIP_DOCKER=1
# skip docker using: make init_db SKIP_DOCKER=0
.PHONY: init_db
init_db:
	@chmod +x ./scripts/init_db.sh
	@./scripts/init_db.sh

.PHONY: migrate
migrate:
	@chmod +x ./scripts/migrate.sh
	@./scripts/migrate.sh
