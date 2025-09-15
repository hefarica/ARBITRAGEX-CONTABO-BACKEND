# Contributing to ArbitrageX Supreme V3.0 Backend

Thank you for your interest in contributing to ArbitrageX Supreme V3.0! This document provides guidelines and information for contributors.

## 🚨 Important Notice

**ArbitrageX Supreme V3.0 is proprietary software.** Contributions are only accepted from authorized team members and approved contractors under signed agreements.

## 📋 Prerequisites

Before contributing, ensure you have:

- Signed the Contributor License Agreement (CLA)
- Access to the private development environment
- Required security clearances
- Development environment properly configured

## 🛠️ Development Setup

### Required Tools

- **Rust**: 1.70+ with cargo
- **Docker**: 20.10+ with Docker Compose
- **PostgreSQL**: 14+
- **Redis**: 6.2+
- **Git**: 2.30+

### Environment Setup

```bash
# Clone the repository (requires access)
git clone https://github.com/hefarica/ARBITRAGEX-CONTABO-BACKEND.git
cd ARBITRAGEX-CONTABO-BACKEND

# Install Rust dependencies
cargo build

# Set up development environment
cp .env.example .env.dev
# Configure your development settings

# Start development services
docker-compose -f docker-compose.dev.yml up -d

# Run database migrations
sqlx migrate run

# Verify setup
cargo test
```

## 🏗️ Architecture Overview

### Core Services

1. **searcher-rs** (Port 8001): Opportunity detection
2. **selector-api** (Port 8002): Opportunity selection
3. **recon** (Port 8003): Reconciliation engine
4. **relays-client** (Port 8004): MEV relay client
5. **api-server** (Port 8000): REST API gateway
6. **sim-ctl** (Port 8005): Simulation controller

### Shared Libraries

- **lib/types.rs**: Common data structures
- **lib/error.rs**: Error handling
- **lib/config.rs**: Configuration management
- **lib/database.rs**: Database operations
- **lib/metrics.rs**: Prometheus metrics
- **lib/utils.rs**: Utility functions

## 📝 Coding Standards

### Rust Guidelines

```rust
// Use explicit error handling
fn process_opportunity(id: Uuid) -> Result<Opportunity, ArbitrageError> {
    // Implementation
}

// Prefer async/await for I/O operations
async fn fetch_price_data(token: &str) -> Result<PriceData, NetworkError> {
    // Implementation
}

// Use structured logging
tracing::info!(
    opportunity_id = %id,
    profit_usd = %profit,
    "Opportunity detected"
);
```

### Code Style

- **Formatting**: Use `cargo fmt` (rustfmt)
- **Linting**: Use `cargo clippy` with strict settings
- **Documentation**: Document all public APIs with `///`
- **Testing**: Maintain >90% test coverage
- **Error Handling**: Use `Result<T, E>` pattern consistently

### Naming Conventions

- **Functions**: `snake_case`
- **Types**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`
- **Files**: `snake_case.rs`

## 🧪 Testing Guidelines

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_opportunity_detection() {
        // Arrange
        let detector = OpportunityDetector::new();
        
        // Act
        let result = detector.detect_opportunities().await;
        
        // Assert
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }
}
```

### Test Categories

- **Unit Tests**: Test individual functions and modules
- **Integration Tests**: Test service interactions
- **Performance Tests**: Benchmark critical paths
- **Security Tests**: Validate security measures

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_opportunity_detection

# Run with coverage
cargo tarpaulin --out Html

# Run integration tests
cargo test --test integration_test

# Run performance benchmarks
cargo bench
```

## 🔄 Development Workflow

### Branch Strategy

- **main**: Production-ready code
- **develop**: Integration branch
- **feature/***: Feature development
- **hotfix/***: Critical fixes
- **release/***: Release preparation

### Commit Guidelines

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
# Feature commits
git commit -m "feat(searcher): add multi-chain opportunity detection"

# Bug fixes
git commit -m "fix(api): resolve race condition in WebSocket handler"

# Documentation
git commit -m "docs(readme): update installation instructions"

# Performance improvements
git commit -m "perf(database): optimize opportunity queries"

# Breaking changes
git commit -m "feat(api)!: change opportunity response format"
```

### Pull Request Process

1. **Create Feature Branch**
   ```bash
   git checkout -b feature/new-feature-name
   ```

2. **Implement Changes**
   - Write code following style guidelines
   - Add comprehensive tests
   - Update documentation
   - Ensure all checks pass

3. **Submit Pull Request**
   - Use descriptive title and description
   - Reference related issues
   - Include testing instructions
   - Add screenshots/logs if applicable

4. **Code Review**
   - Address reviewer feedback
   - Ensure CI/CD passes
   - Maintain clean commit history

5. **Merge Requirements**
   - ✅ All tests passing
   - ✅ Code review approved
   - ✅ Security scan passed
   - ✅ Performance benchmarks met
   - ✅ Documentation updated

## 🔒 Security Guidelines

### Sensitive Data

- **Never commit secrets** (API keys, passwords, private keys)
- **Use environment variables** for configuration
- **Encrypt sensitive data** at rest and in transit
- **Follow principle of least privilege**

### Code Security

```rust
// ✅ Good: Use parameterized queries
sqlx::query!("SELECT * FROM opportunities WHERE id = $1", id)
    .fetch_one(&pool).await?;

// ❌ Bad: SQL injection risk
format!("SELECT * FROM opportunities WHERE id = '{}'", id)
```

### Security Checklist

- [ ] Input validation implemented
- [ ] SQL injection prevention
- [ ] XSS protection in APIs
- [ ] Rate limiting configured
- [ ] Authentication/authorization tested
- [ ] Dependency vulnerabilities scanned

## 📊 Performance Guidelines

### Optimization Principles

- **Measure first**: Use profiling tools
- **Optimize hot paths**: Focus on critical performance areas
- **Async operations**: Use async/await for I/O
- **Connection pooling**: Reuse database connections
- **Caching**: Cache frequently accessed data

### Performance Targets

- **API Response Time**: < 100ms (95th percentile)
- **Opportunity Detection**: < 50ms per opportunity
- **Database Queries**: < 10ms average
- **Memory Usage**: < 512MB per service
- **CPU Usage**: < 50% average load

### Monitoring

```rust
// Add metrics to critical paths
metrics::counter!("opportunities_detected_total").increment(1);
metrics::histogram!("opportunity_processing_duration_ms").record(duration.as_millis() as f64);
```

## 🐛 Debugging Guidelines

### Logging

```rust
// Use structured logging
tracing::info!(
    user_id = %user_id,
    opportunity_id = %opp_id,
    profit_usd = %profit,
    "Opportunity executed successfully"
);

// Log errors with context
tracing::error!(
    error = %err,
    opportunity_id = %opp_id,
    "Failed to execute opportunity"
);
```

### Debug Tools

- **Rust Analyzer**: IDE integration
- **cargo-expand**: Macro expansion
- **flamegraph**: CPU profiling
- **heaptrack**: Memory profiling
- **strace**: System call tracing

## 📚 Documentation

### Code Documentation

```rust
/// Detects arbitrage opportunities across multiple DEXs
/// 
/// # Arguments
/// 
/// * `chains` - List of blockchain networks to scan
/// * `min_profit` - Minimum profit threshold in USD
/// 
/// # Returns
/// 
/// Vector of detected opportunities sorted by profitability
/// 
/// # Errors
/// 
/// Returns `DetectionError` if network requests fail
pub async fn detect_opportunities(
    chains: &[ChainId],
    min_profit: Decimal
) -> Result<Vec<Opportunity>, DetectionError> {
    // Implementation
}
```

### API Documentation

- Use OpenAPI/Swagger specifications
- Include request/response examples
- Document error codes and messages
- Provide integration examples

## 🚀 Deployment

### Development Deployment

```bash
# Build and test
cargo build --release
cargo test

# Deploy to development environment
./scripts/deploy.sh development

# Verify deployment
./scripts/health-check.sh development
```

### Production Deployment

- **Staging First**: Always deploy to staging
- **Health Checks**: Verify all services healthy
- **Rollback Plan**: Have rollback procedure ready
- **Monitoring**: Watch metrics during deployment

## 📞 Support and Communication

### Internal Communication

- **Slack**: #arbitragex-dev for development discussions
- **Email**: dev-team@arbitragex.io for formal communications
- **Meetings**: Weekly standup Mondays 9 AM UTC

### Issue Reporting

When reporting issues, include:

- **Environment**: Development/Staging/Production
- **Service**: Which service is affected
- **Steps to Reproduce**: Detailed reproduction steps
- **Expected vs Actual**: What should happen vs what happens
- **Logs**: Relevant log entries
- **Impact**: Business impact assessment

### Emergency Procedures

For production issues:

1. **Immediate Response**: Contact on-call engineer
2. **Assessment**: Evaluate impact and severity
3. **Mitigation**: Implement immediate fixes
4. **Communication**: Update stakeholders
5. **Post-Mortem**: Conduct thorough analysis

## 📋 Checklist for Contributors

Before submitting code:

- [ ] Code follows style guidelines
- [ ] Tests written and passing
- [ ] Documentation updated
- [ ] Security review completed
- [ ] Performance impact assessed
- [ ] Breaking changes documented
- [ ] Migration scripts provided (if needed)

## 🏆 Recognition

Outstanding contributions are recognized through:

- **Code Quality Awards**: Monthly recognition
- **Innovation Bonuses**: For significant improvements
- **Conference Speaking**: Opportunities to present work
- **Career Advancement**: Contribution-based promotions

---

## 📄 Legal Notice

By contributing to this project, you agree that your contributions will be licensed under the same proprietary license as the project. All contributors must sign the Contributor License Agreement before their first contribution.

**This is proprietary software. Unauthorized access, use, or distribution is strictly prohibited.**

For questions about contributing, contact: dev-team@arbitragex.io
