.PHONY: all build run test test-unit test-integration test-e2e test-all lint clean docker-build docker-run docker-compose-up docker-compose-down coverage

APP_NAME := metaclouds-backend
BUILD_DIR := build
GO_FILES := $(shell find . -name "*.go" -type f)

all: build

build:
	@mkdir -p $(BUILD_DIR)
	go build -o $(BUILD_DIR)/$(APP_NAME) .

run: build
	$(BUILD_DIR)/$(APP_NAME)

test: test-unit

test-unit:
	go test -v -race ./services/... ./controllers/... ./models/... ./pkg/...

test-integration:
	go test -v -race -tags=integration ./tests/...

test-e2e:
	go test -v -race ./tests/e2e_test.go -run TestE2E -timeout 120s

test-all: test-unit test-integration test-e2e

test-coverage:
	go test -race -coverprofile=coverage.out ./...
	go tool cover -func=coverage.out

test-coverage-html:
	go test -race -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html

lint:
	golangci-lint run ./...

lint-fix:
	golangci-lint run --fix ./...

fmt:
	go fmt ./...

vet:
	go vet ./...

clean:
	rm -rf $(BUILD_DIR)
	rm -f $(APP_NAME)
	rm -f coverage.out
	rm -f coverage.html

docker-build:
	docker build -t $(APP_NAME):latest .

docker-run: docker-build
	docker run -p 8000:8000 $(APP_NAME):latest

docker-compose-up:
	docker-compose up -d

docker-compose-down:
	docker-compose down

docker-compose-logs:
	docker-compose logs -f

docker-compose-restart:
	docker-compose restart $(APP_NAME)

deps:
	go mod download
	go mod tidy

deps-update:
	go get -u ./...
	go mod tidy

generate:
	go generate ./...

check: fmt vet lint test

release: check build
	@echo "Build completed successfully"

help:
	@echo "Available targets:"
	@echo "  all              - Build the application"
	@echo "  build            - Build the application"
	@echo "  run              - Build and run the application"
	@echo "  test             - Run all tests"
	@echo "  test-unit        - Run unit tests"
	@echo "  test-integration - Run integration tests"
	@echo "  test-e2e         - Run end-to-end tests"
	@echo "  test-all         - Run all tests (unit + integration + e2e)"
	@echo "  test-coverage    - Run tests with coverage"
	@echo "  test-coverage-html - Run tests with coverage and generate HTML report"
	@echo "  lint             - Run golangci-lint"
	@echo "  lint-fix         - Run golangci-lint with --fix"
	@echo "  fmt              - Format code"
	@echo "  vet              - Run go vet"
	@echo "  clean            - Clean build artifacts"
	@echo "  docker-build     - Build Docker image"
	@echo "  docker-run       - Run Docker container"
	@echo "  docker-compose-up   - Start all services with docker-compose"
	@echo "  docker-compose-down - Stop all services with docker-compose"
	@echo "  docker-compose-logs - View logs from all services"
	@echo "  deps             - Download dependencies"
	@echo "  deps-update      - Update dependencies"
	@echo "  generate         - Run go generate"
	@echo "  check            - Run fmt, vet, lint, and test"
	@echo "  release          - Run check and build"
	@echo "  help             - Show this help message"
