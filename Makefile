APP_NAME=zero2prod

.PHONY: run
run:
	@cargo watch -x check -x test -x run

.PHONY: test
test:
	@cargo test

.PHONY: init_db
init_db:
	@chmod +x ./scripts/init_db.sh
	@./scripts/init_db.sh

.PHONY: migrate
migrate:
	@chmod +x ./scripts/migrate.sh
	@./scripts/migrate.sh

.PHONY: docker_build
docker_build:
	@docker build --tag $(APP_NAME) --file Dockerfile .

.PHONY: docker_run
docker_run:
	@docker run -p 18000:18000 $(APP_NAME)
