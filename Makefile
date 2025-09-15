# ArbitrageX Supreme V3.0 - Backend Makefile
# Automation for development, testing, and deployment

.PHONY: help install build test clean dev prod deploy health lint format check security docs

# Default target
help: ## Show this help message
	@echo "ArbitrageX Supreme V3.0 - Backend Makefile"
	@echo "=========================================="
	@echo ""
	@echo "Available targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# Installation and Setup
install: ## Install all dependencies and set up development environment
	@echo "🔧 Installing Rust dependencies..."
	cargo build
	@echo "🐳 Setting up Docker services..."
	docker-compose -f docker-compose.dev.yml pull
	@echo "📦 Installing additional tools..."
	cargo install sqlx-cli --no-default-features --features postgres
	cargo install cargo-tarpaulin
	cargo install cargo-audit
	@echo "✅ Installation complete!"

setup-db: ## Set up database and run migrations
	@echo "🗄️ Setting up PostgreSQL database..."
	docker-compose -f docker-compose.dev.yml up -d postgres redis
	sleep 5
	sqlx migrate run
	@echo "✅ Database setup complete!"

# Development
dev: ## Start development environment
	@echo "🚀 Starting development environment..."
	docker-compose -f docker-compose.dev.yml up -d
	@echo "✅ Development environment started!"
	@echo "📊 Services available at:"
	@echo "  - API Server: http://localhost:8000"
	@echo "  - Searcher: http://localhost:8001"
	@echo "  - Selector: http://localhost:8002"
	@echo "  - Recon: http://localhost:8003"
	@echo "  - Relays: http://localhost:8004"
	@echo "  - Sim-CTL: http://localhost:8005"
	@echo "  - Prometheus: http://localhost:9090"
	@echo "  - Grafana: http://localhost:3000"

dev-stop: ## Stop development environment
	@echo "🛑 Stopping development environment..."
	docker-compose -f docker-compose.dev.yml down
	@echo "✅ Development environment stopped!"

dev-logs: ## Show development logs
	docker-compose -f docker-compose.dev.yml logs -f

# Building
build: ## Build all services in release mode
	@echo "🔨 Building all services..."
	cargo build --release
	@echo "✅ Build complete!"

build-debug: ## Build all services in debug mode
	@echo "🔨 Building all services (debug)..."
	cargo build
	@echo "✅ Debug build complete!"

build-docker: ## Build Docker images for all services
	@echo "🐳 Building Docker images..."
	docker build -f Dockerfile.searcher -t arbitragex/searcher:latest .
	docker build -f Dockerfile.api -t arbitragex/api-server:latest .
	docker build -f Dockerfile.selector-api -t arbitragex/selector-api:latest .
	docker build -f Dockerfile.recon -t arbitragex/recon:latest .
	docker build -f Dockerfile.relays-client -t arbitragex/relays-client:latest .
	docker build -f Dockerfile.sim-ctl -t arbitragex/sim-ctl:latest .
	@echo "✅ Docker images built!"

# Testing
test: ## Run all tests
	@echo "🧪 Running tests..."
	cargo test
	@echo "✅ Tests complete!"

test-coverage: ## Run tests with coverage report
	@echo "📊 Running tests with coverage..."
	cargo tarpaulin --out Html --output-dir coverage
	@echo "✅ Coverage report generated in coverage/"

test-integration: ## Run integration tests
	@echo "🔗 Running integration tests..."
	cargo test --test integration_test
	@echo "✅ Integration tests complete!"

test-performance: ## Run performance benchmarks
	@echo "⚡ Running performance benchmarks..."
	cargo bench
	@echo "✅ Benchmarks complete!"

# Code Quality
lint: ## Run linter (clippy)
	@echo "🔍 Running linter..."
	cargo clippy -- -D warnings
	@echo "✅ Linting complete!"

format: ## Format code
	@echo "🎨 Formatting code..."
	cargo fmt
	@echo "✅ Code formatted!"

format-check: ## Check code formatting
	@echo "🎨 Checking code formatting..."
	cargo fmt -- --check
	@echo "✅ Format check complete!"

check: ## Run all checks (build, test, lint, format)
	@echo "✅ Running all checks..."
	$(MAKE) build
	$(MAKE) test
	$(MAKE) lint
	$(MAKE) format-check
	@echo "🎉 All checks passed!"

# Security
security: ## Run security audit
	@echo "🔒 Running security audit..."
	cargo audit
	@echo "✅ Security audit complete!"

security-deps: ## Update dependencies and check for vulnerabilities
	@echo "📦 Updating dependencies..."
	cargo update
	cargo audit
	@echo "✅ Dependencies updated and audited!"

# Production
prod: ## Start production environment
	@echo "🚀 Starting production environment..."
	docker-compose -f docker-compose.prod.yml up -d
	@echo "✅ Production environment started!"

prod-stop: ## Stop production environment
	@echo "🛑 Stopping production environment..."
	docker-compose -f docker-compose.prod.yml down
	@echo "✅ Production environment stopped!"

prod-logs: ## Show production logs
	docker-compose -f docker-compose.prod.yml logs -f

# Deployment
deploy-dev: ## Deploy to development environment
	@echo "🚀 Deploying to development..."
	./scripts/deploy.sh development
	@echo "✅ Development deployment complete!"

deploy-staging: ## Deploy to staging environment
	@echo "🚀 Deploying to staging..."
	./scripts/deploy.sh staging
	@echo "✅ Staging deployment complete!"

deploy-prod: ## Deploy to production environment
	@echo "🚀 Deploying to production..."
	./scripts/deploy.sh production
	@echo "✅ Production deployment complete!"

# Health Checks
health: ## Check health of all services
	@echo "🏥 Checking service health..."
	@curl -f http://localhost:8000/health || echo "❌ API Server unhealthy"
	@curl -f http://localhost:8001/health || echo "❌ Searcher unhealthy"
	@curl -f http://localhost:8002/health || echo "❌ Selector unhealthy"
	@curl -f http://localhost:8003/health || echo "❌ Recon unhealthy"
	@curl -f http://localhost:8004/health || echo "❌ Relays unhealthy"
	@curl -f http://localhost:8005/health || echo "❌ Sim-CTL unhealthy"
	@echo "✅ Health check complete!"

health-detailed: ## Detailed health check with metrics
	@echo "🏥 Detailed health check..."
	@./scripts/health-check.sh

# Database Operations
db-migrate: ## Run database migrations
	@echo "🗄️ Running database migrations..."
	sqlx migrate run
	@echo "✅ Migrations complete!"

db-rollback: ## Rollback last migration
	@echo "🔄 Rolling back last migration..."
	sqlx migrate revert
	@echo "✅ Rollback complete!"

db-reset: ## Reset database (development only)
	@echo "⚠️  Resetting database (development only)..."
	@read -p "Are you sure? This will delete all data! (y/N): " confirm && [ "$$confirm" = "y" ]
	docker-compose -f docker-compose.dev.yml down -v
	docker-compose -f docker-compose.dev.yml up -d postgres redis
	sleep 5
	sqlx migrate run
	@echo "✅ Database reset complete!"

# Monitoring
metrics: ## Show Prometheus metrics
	@echo "📊 Fetching Prometheus metrics..."
	@curl -s http://localhost:9090/api/v1/query?query=up | jq .

logs: ## Show aggregated logs
	@echo "📋 Showing logs..."
	docker-compose logs -f --tail=100

logs-service: ## Show logs for specific service (usage: make logs-service SERVICE=searcher)
	@echo "📋 Showing logs for $(SERVICE)..."
	docker-compose logs -f --tail=100 $(SERVICE)

# Documentation
docs: ## Generate documentation
	@echo "📚 Generating documentation..."
	cargo doc --no-deps --open
	@echo "✅ Documentation generated!"

docs-api: ## Generate API documentation
	@echo "📚 Generating API documentation..."
	@echo "API documentation available at: http://localhost:8000/docs"

# Utilities
clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	docker system prune -f
	@echo "✅ Cleanup complete!"

clean-all: ## Clean everything including Docker volumes
	@echo "🧹 Deep cleaning..."
	cargo clean
	docker-compose -f docker-compose.dev.yml down -v
	docker-compose -f docker-compose.prod.yml down -v
	docker system prune -af --volumes
	@echo "✅ Deep cleanup complete!"

backup: ## Create backup of database and configuration
	@echo "💾 Creating backup..."
	./scripts/backup.sh
	@echo "✅ Backup complete!"

restore: ## Restore from backup (usage: make restore BACKUP_FILE=backup.sql)
	@echo "🔄 Restoring from backup..."
	./scripts/restore.sh $(BACKUP_FILE)
	@echo "✅ Restore complete!"

# Environment Management
env-dev: ## Set up development environment variables
	@echo "🔧 Setting up development environment..."
	cp .env.example .env.dev
	@echo "✅ Edit .env.dev with your development settings"

env-prod: ## Set up production environment variables
	@echo "🔧 Setting up production environment..."
	cp .env.example .env.prod
	@echo "⚠️  Edit .env.prod with your production settings"

# Performance
benchmark: ## Run performance benchmarks
	@echo "⚡ Running performance benchmarks..."
	cargo bench --bench opportunity_detection
	cargo bench --bench execution_speed
	@echo "✅ Benchmarks complete!"

profile: ## Profile application performance
	@echo "📊 Profiling application..."
	cargo build --release
	perf record --call-graph=dwarf target/release/searcher-rs
	perf report
	@echo "✅ Profiling complete!"

# Development Tools
watch: ## Watch for changes and rebuild
	@echo "👀 Watching for changes..."
	cargo watch -x build

watch-test: ## Watch for changes and run tests
	@echo "👀 Watching for changes and running tests..."
	cargo watch -x test

watch-run: ## Watch for changes and run specific service
	@echo "👀 Watching for changes and running $(SERVICE)..."
	cargo watch -x "run --bin $(SERVICE)"

# Git Hooks
install-hooks: ## Install Git hooks
	@echo "🪝 Installing Git hooks..."
	cp scripts/git-hooks/pre-commit .git/hooks/
	cp scripts/git-hooks/pre-push .git/hooks/
	chmod +x .git/hooks/pre-commit
	chmod +x .git/hooks/pre-push
	@echo "✅ Git hooks installed!"

# Quick Commands
quick-start: install setup-db dev ## Quick start for new developers
	@echo "🎉 Quick start complete! ArbitrageX is ready for development."

quick-test: build test lint ## Quick test suite
	@echo "🎉 Quick test complete!"

quick-deploy: check build-docker deploy-staging ## Quick deployment to staging
	@echo "🎉 Quick deployment complete!"

# Status
status: ## Show system status
	@echo "📊 ArbitrageX System Status"
	@echo "=========================="
	@echo ""
	@echo "🐳 Docker Services:"
	@docker-compose ps
	@echo ""
	@echo "🏥 Service Health:"
	@$(MAKE) health
	@echo ""
	@echo "📊 Resource Usage:"
	@docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"

# Version Information
version: ## Show version information
	@echo "ArbitrageX Supreme V3.0 - Backend"
	@echo "================================"
	@echo "Rust Version: $$(rustc --version)"
	@echo "Cargo Version: $$(cargo --version)"
	@echo "Docker Version: $$(docker --version)"
	@echo "Docker Compose Version: $$(docker-compose --version)"
	@echo "PostgreSQL Version: $$(docker-compose exec postgres psql --version 2>/dev/null || echo 'Not running')"
	@echo "Redis Version: $$(docker-compose exec redis redis-server --version 2>/dev/null || echo 'Not running')"
