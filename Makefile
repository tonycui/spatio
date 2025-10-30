.PHONY: help build run test clean docker-build docker-run docker-push docker-compose-up docker-compose-down fmt clippy

# 默认目标
.DEFAULT_GOAL := help

# 颜色定义
GREEN  := \033[0;32m
YELLOW := \033[0;33m
BLUE   := \033[0;34m
NC     := \033[0m # No Color

# Docker 配置
DOCKER_IMAGE := spaito/spatio
DOCKER_TAG := latest

help: ## 显示帮助信息
	@echo "$(BLUE)Spatio Makefile Commands:$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Examples:$(NC)"
	@echo "  make build          # 构建项目"
	@echo "  make docker-build   # 构建 Docker 镜像"
	@echo "  make test           # 运行测试"

# ==================== Rust 构建 ====================

build: ## 构建 release 版本
	@echo "$(GREEN)Building Spatio...$(NC)"
	cargo build --release

build-debug: ## 构建 debug 版本
	@echo "$(GREEN)Building Spatio (debug)...$(NC)"
	cargo build

run: ## 运行 Spatio server
	@echo "$(GREEN)Starting Spatio server...$(NC)"
	cargo run --release --bin spatio-server

run-cli: ## 运行 Spatio CLI (交互模式)
	@echo "$(GREEN)Starting Spatio CLI...$(NC)"
	cargo run --release --bin spatio-cli -- --interactive

# ==================== 测试 ====================

test: ## 运行所有测试
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test

test-verbose: ## 运行测试（详细输出）
	@echo "$(GREEN)Running tests (verbose)...$(NC)"
	cargo test -- --nocapture

test-one: ## 运行单个测试 (使用: make test-one TEST=test_name)
	@echo "$(GREEN)Running test: $(TEST)$(NC)"
	cargo test $(TEST) -- --nocapture

bench: ## 运行性能测试
	@echo "$(GREEN)Running benchmarks...$(NC)"
	python3 benchmark/benchmark_geo42_only_concurrent.py

# ==================== 代码质量 ====================

fmt: ## 格式化代码
	@echo "$(GREEN)Formatting code...$(NC)"
	cargo fmt

fmt-check: ## 检查代码格式
	@echo "$(GREEN)Checking code format...$(NC)"
	cargo fmt -- --check

clippy: ## 运行 Clippy 检查
	@echo "$(GREEN)Running Clippy...$(NC)"
	cargo clippy -- -D warnings

clippy-fix: ## 自动修复 Clippy 警告
	@echo "$(GREEN)Fixing Clippy warnings...$(NC)"
	cargo clippy --fix

check: ## 检查代码（快速检查编译错误）
	@echo "$(GREEN)Checking code...$(NC)"
	cargo check

# ==================== 清理 ====================

clean: ## 清理构建产物
	@echo "$(YELLOW)Cleaning build artifacts...$(NC)"
	cargo clean

clean-all: clean docker-clean ## 清理所有（包括 Docker）
	@echo "$(YELLOW)Cleaned everything!$(NC)"

# ==================== Docker ====================

docker-build: ## 构建 Docker 镜像
	@echo "$(GREEN)Building Docker image...$(NC)"
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .

docker-build-local: ## 本地测试构建 Docker 镜像（用于验证 CI）
	@echo "$(GREEN)Building Docker image locally (CI simulation)...$(NC)"
	@echo "$(YELLOW)This simulates the GitHub Actions build process$(NC)"
	docker build -t spatio:local-test .

docker-build-no-cache: ## 构建 Docker 镜像（不使用缓存）
	@echo "$(GREEN)Building Docker image (no cache)...$(NC)"
	docker build --no-cache -t $(DOCKER_IMAGE):$(DOCKER_TAG) .

docker-run: ## 运行 Docker 容器
	@echo "$(GREEN)Running Docker container...$(NC)"
	docker run -p 9851:9851 --name spatio $(DOCKER_IMAGE):$(DOCKER_TAG)

docker-run-detach: ## 后台运行 Docker 容器
	@echo "$(GREEN)Running Docker container (detached)...$(NC)"
	docker run -d -p 9851:9851 --name spatio $(DOCKER_IMAGE):$(DOCKER_TAG)

docker-stop: ## 停止 Docker 容器
	@echo "$(YELLOW)Stopping Docker container...$(NC)"
	docker stop spatio || true
	docker rm spatio || true

docker-logs: ## 查看 Docker 日志
	@echo "$(GREEN)Showing Docker logs...$(NC)"
	docker logs -f spatio

docker-shell: ## 进入 Docker 容器 shell
	@echo "$(GREEN)Entering Docker container...$(NC)"
	docker exec -it spatio /bin/sh

docker-push: ## 推送 Docker 镜像到 Docker Hub
	@echo "$(GREEN)Pushing Docker image...$(NC)"
	docker push $(DOCKER_IMAGE):$(DOCKER_TAG)

docker-clean: ## 清理 Docker 镜像和容器
	@echo "$(YELLOW)Cleaning Docker artifacts...$(NC)"
	docker stop spatio || true
	docker rm spatio || true
	docker rmi $(DOCKER_IMAGE):$(DOCKER_TAG) || true
	docker system prune -f

# ==================== Docker Compose ====================

docker-compose-up: ## 启动 docker-compose
	@echo "$(GREEN)Starting with docker-compose...$(NC)"
	docker-compose up -d

docker-compose-down: ## 停止 docker-compose
	@echo "$(YELLOW)Stopping docker-compose...$(NC)"
	docker-compose down

docker-compose-logs: ## 查看 docker-compose 日志
	@echo "$(GREEN)Showing docker-compose logs...$(NC)"
	docker-compose logs -f

docker-compose-rebuild: ## 重新构建并启动 docker-compose
	@echo "$(GREEN)Rebuilding with docker-compose...$(NC)"
	docker-compose up -d --build

# ==================== 开发工具 ====================

dev: ## 启动开发环境（服务器 + 监控）
	@echo "$(GREEN)Starting development environment...$(NC)"
	cargo watch -x 'run --bin spatio-server'

install-tools: ## 安装开发工具
	@echo "$(GREEN)Installing development tools...$(NC)"
	cargo install cargo-watch
	cargo install cargo-edit

update-deps: ## 更新依赖
	@echo "$(GREEN)Updating dependencies...$(NC)"
	cargo update

# ==================== 文档 ====================

doc: ## 生成并打开文档
	@echo "$(GREEN)Generating documentation...$(NC)"
	cargo doc --open

doc-no-deps: ## 生成文档（不包含依赖）
	@echo "$(GREEN)Generating documentation (no deps)...$(NC)"
	cargo doc --no-deps --open

# ==================== 发布 ====================

publish-check: ## 检查是否可以发布
	@echo "$(GREEN)Checking for publish...$(NC)"
	cargo publish --dry-run

# ==================== 快捷命令组合 ====================

all: clean build test ## 清理、构建、测试

ci: fmt-check clippy test ## CI 流程：格式检查、Clippy、测试

docker-all: docker-build docker-run ## 构建并运行 Docker

quick-test: build test ## 快速构建和测试

# ==================== 版本信息 ====================

version: ## 显示版本信息
	@echo "$(BLUE)Rust version:$(NC)"
	@rustc --version
	@echo "$(BLUE)Cargo version:$(NC)"
	@cargo --version
	@echo "$(BLUE)Docker version:$(NC)"
	@docker --version || echo "Docker not installed"
