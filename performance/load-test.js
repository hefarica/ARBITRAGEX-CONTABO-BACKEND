// ArbitrageX Supreme V3.0 - Performance Testing Suite
// Ubicación: performance/load-test.js (Backend) y performance/edge-load-test.js (Edge)
// Paquete Operativo Completo - K6 Performance Tests

import http from 'k6/http';
import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const responseTime = new Trend('response_time');
const wsConnections = new Counter('websocket_connections');
const wsMessages = new Counter('websocket_messages');

// Test configuration
const BASE_URL = __ENV.BASE_URL || 'https://api.arbitragex.dev';
const EDGE_URL = __ENV.EDGE_URL || 'https://arbitragex.workers.dev';
const WS_URL = __ENV.WS_URL || 'wss://arbitragex.workers.dev/ws';

// Load test scenarios
export const options = {
  scenarios: {
    // Smoke test - Basic functionality
    smoke_test: {
      executor: 'constant-vus',
      vus: 1,
      duration: '30s',
      tags: { test_type: 'smoke' },
    },
    
    // Load test - Normal traffic
    load_test: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '2m', target: 20 },   // Ramp up
        { duration: '5m', target: 20 },   // Stay at 20 users
        { duration: '2m', target: 50 },   // Ramp to 50 users
        { duration: '5m', target: 50 },   // Stay at 50 users
        { duration: '2m', target: 0 },    // Ramp down
      ],
      tags: { test_type: 'load' },
    },
    
    // Stress test - High traffic
    stress_test: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '2m', target: 100 },  // Ramp up to 100 users
        { duration: '5m', target: 100 },  // Stay at 100 users
        { duration: '2m', target: 200 },  // Ramp to 200 users
        { duration: '5m', target: 200 },  // Stay at 200 users
        { duration: '10m', target: 0 },   // Ramp down
      ],
      tags: { test_type: 'stress' },
    },
    
    // Spike test - Sudden traffic spikes
    spike_test: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '10s', target: 100 }, // Spike to 100 users
        { duration: '1m', target: 100 },  // Stay at 100 users
        { duration: '10s', target: 1000 }, // Spike to 1000 users
        { duration: '3m', target: 1000 }, // Stay at 1000 users
        { duration: '10s', target: 100 }, // Drop back to 100
        { duration: '3m', target: 100 },  // Stay at 100
        { duration: '10s', target: 0 },   // Ramp down
      ],
      tags: { test_type: 'spike' },
    },
    
    // WebSocket test - Real-time connections
    websocket_test: {
      executor: 'constant-vus',
      vus: 50,
      duration: '5m',
      tags: { test_type: 'websocket' },
    },
  },
  
  // Thresholds - SLOs
  thresholds: {
    http_req_duration: ['p(95)<500'], // 95% of requests under 500ms
    http_req_failed: ['rate<0.01'],   // Error rate under 1%
    errors: ['rate<0.01'],            // Custom error rate under 1%
    response_time: ['p(95)<200'],     // 95% response time under 200ms
    websocket_connections: ['count>0'], // At least some WS connections
  },
};

// =====================================================
// BACKEND API TESTS
// =====================================================

export function backend_health_test() {
  const response = http.get(`${BASE_URL}/health`);
  
  check(response, {
    'health check status is 200': (r) => r.status === 200,
    'health check response time < 100ms': (r) => r.timings.duration < 100,
    'health check has correct body': (r) => {
      const body = JSON.parse(r.body);
      return body.status === 'healthy';
    },
  });
  
  errorRate.add(response.status !== 200);
  responseTime.add(response.timings.duration);
}

export function backend_opportunities_test() {
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
      'Authorization': 'Bearer test-token',
    },
  };
  
  const response = http.get(`${BASE_URL}/api/opportunities`, params);
  
  check(response, {
    'opportunities status is 200': (r) => r.status === 200,
    'opportunities response time < 500ms': (r) => r.timings.duration < 500,
    'opportunities returns array': (r) => {
      try {
        const body = JSON.parse(r.body);
        return Array.isArray(body.data || body);
      } catch (e) {
        return false;
      }
    },
    'opportunities has valid structure': (r) => {
      try {
        const body = JSON.parse(r.body);
        const opportunities = body.data || body;
        if (opportunities.length > 0) {
          const first = opportunities[0];
          return first.hasOwnProperty('pair_address') && 
                 first.hasOwnProperty('profit_potential') &&
                 first.hasOwnProperty('dex_a') &&
                 first.hasOwnProperty('dex_b');
        }
        return true; // Empty array is valid
      } catch (e) {
        return false;
      }
    },
  });
  
  errorRate.add(response.status !== 200);
  responseTime.add(response.timings.duration);
}

export function backend_executions_test() {
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer test-token',
    },
  };
  
  const response = http.get(`${BASE_URL}/api/executions`, params);
  
  check(response, {
    'executions status is 200': (r) => r.status === 200,
    'executions response time < 500ms': (r) => r.timings.duration < 500,
    'executions returns array': (r) => {
      try {
        const body = JSON.parse(r.body);
        return Array.isArray(body.data || body);
      } catch (e) {
        return false;
      }
    },
  });
  
  errorRate.add(response.status !== 200);
  responseTime.add(response.timings.duration);
}

export function backend_analytics_test() {
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer test-token',
    },
  };
  
  const response = http.get(`${BASE_URL}/api/analytics`, params);
  
  check(response, {
    'analytics status is 200': (r) => r.status === 200,
    'analytics response time < 500ms': (r) => r.timings.duration < 500,
    'analytics has metrics': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.hasOwnProperty('total_profit') ||
               body.hasOwnProperty('success_rate') ||
               body.hasOwnProperty('total_volume');
      } catch (e) {
        return false;
      }
    },
  });
  
  errorRate.add(response.status !== 200);
  responseTime.add(response.timings.duration);
}

// =====================================================
// EDGE WORKERS TESTS
// =====================================================

export function edge_health_test() {
  const response = http.get(`${EDGE_URL}/health`);
  
  check(response, {
    'edge health status is 200': (r) => r.status === 200,
    'edge health response time < 50ms': (r) => r.timings.duration < 50,
    'edge health from correct location': (r) => {
      const cfRay = r.headers['Cf-Ray'];
      return cfRay !== undefined; // Cloudflare header present
    },
  });
  
  errorRate.add(response.status !== 200);
  responseTime.add(response.timings.duration);
}

export function edge_opportunities_proxy_test() {
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer test-token',
    },
  };
  
  const response = http.get(`${EDGE_URL}/api/opportunities`, params);
  
  check(response, {
    'edge opportunities status is 200': (r) => r.status === 200,
    'edge opportunities response time < 100ms': (r) => r.timings.duration < 100,
    'edge opportunities cached': (r) => {
      const cacheStatus = r.headers['Cf-Cache-Status'];
      return cacheStatus === 'HIT' || cacheStatus === 'MISS' || cacheStatus === 'DYNAMIC';
    },
    'edge opportunities rate limited': (r) => {
      const rateLimitRemaining = r.headers['X-RateLimit-Remaining'];
      return rateLimitRemaining !== undefined;
    },
  });
  
  errorRate.add(response.status !== 200);
  responseTime.add(response.timings.duration);
}

export function edge_rate_limiting_test() {
  // Test rate limiting by making rapid requests
  const requests = [];
  for (let i = 0; i < 10; i++) {
    requests.push(['GET', `${EDGE_URL}/api/opportunities`, null, {
      headers: { 'Authorization': 'Bearer test-token' }
    }]);
  }
  
  const responses = http.batch(requests);
  
  let rateLimitHit = false;
  responses.forEach((response, index) => {
    if (response.status === 429) {
      rateLimitHit = true;
    }
    
    check(response, {
      [`batch request ${index} has valid status`]: (r) => r.status === 200 || r.status === 429,
      [`batch request ${index} has rate limit headers`]: (r) => {
        return r.headers['X-RateLimit-Remaining'] !== undefined &&
               r.headers['X-RateLimit-Reset'] !== undefined;
      },
    });
  });
  
  check({ rateLimitHit }, {
    'rate limiting is working': (obj) => obj.rateLimitHit === true,
  });
}

// =====================================================
// WEBSOCKET TESTS
// =====================================================

export function websocket_connection_test() {
  const url = WS_URL;
  let messageCount = 0;
  
  const res = ws.connect(url, {}, function (socket) {
    wsConnections.add(1);
    
    socket.on('open', () => {
      console.log('WebSocket connected');
      
      // Send authentication if needed
      socket.send(JSON.stringify({
        type: 'auth',
        token: 'test-token'
      }));
    });
    
    socket.on('message', (data) => {
      messageCount++;
      wsMessages.add(1);
      
      try {
        const message = JSON.parse(data);
        
        check(message, {
          'websocket message has type': (msg) => msg.hasOwnProperty('type'),
          'websocket message has data': (msg) => msg.hasOwnProperty('data'),
          'websocket message has timestamp': (msg) => msg.hasOwnProperty('timestamp'),
        });
        
        // Log different message types
        if (message.type === 'opportunity_update') {
          console.log('Received opportunity update');
        } else if (message.type === 'execution_update') {
          console.log('Received execution update');
        }
        
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    });
    
    socket.on('close', () => {
      console.log('WebSocket disconnected');
    });
    
    socket.on('error', (e) => {
      console.error('WebSocket error:', e);
    });
    
    // Keep connection alive for 30 seconds
    socket.setTimeout(() => {
      socket.close();
    }, 30000);
  });
  
  check(res, {
    'websocket connection established': (r) => r && r.url === url,
  });
  
  // Wait for messages
  sleep(30);
  
  check({ messageCount }, {
    'websocket received messages': (obj) => obj.messageCount > 0,
    'websocket received multiple messages': (obj) => obj.messageCount > 5,
  });
}

// =====================================================
// MAIN TEST FUNCTION
// =====================================================

export default function () {
  const testType = __ENV.TEST_TYPE || 'load';
  
  switch (testType) {
    case 'smoke':
      backend_health_test();
      edge_health_test();
      break;
      
    case 'load':
      // Distribute load across different endpoints
      const rand = Math.random();
      if (rand < 0.3) {
        backend_opportunities_test();
      } else if (rand < 0.6) {
        edge_opportunities_proxy_test();
      } else if (rand < 0.8) {
        backend_executions_test();
      } else {
        backend_analytics_test();
      }
      break;
      
    case 'stress':
      // Focus on high-traffic endpoints
      if (Math.random() < 0.7) {
        edge_opportunities_proxy_test();
      } else {
        backend_opportunities_test();
      }
      break;
      
    case 'spike':
      // Simulate sudden spikes
      edge_opportunities_proxy_test();
      backend_opportunities_test();
      break;
      
    case 'websocket':
      websocket_connection_test();
      return; // Don't sleep for WebSocket tests
      
    default:
      backend_health_test();
      edge_health_test();
  }
  
  // Random sleep between 1-3 seconds
  sleep(Math.random() * 2 + 1);
}

// =====================================================
// SETUP AND TEARDOWN
// =====================================================

export function setup() {
  console.log('🚀 Starting ArbitrageX Performance Tests');
  console.log(`Backend URL: ${BASE_URL}`);
  console.log(`Edge URL: ${EDGE_URL}`);
  console.log(`WebSocket URL: ${WS_URL}`);
  
  // Warm up the services
  const warmupRequests = [
    ['GET', `${BASE_URL}/health`],
    ['GET', `${EDGE_URL}/health`],
  ];
  
  const responses = http.batch(warmupRequests);
  responses.forEach((response, index) => {
    console.log(`Warmup request ${index}: ${response.status}`);
  });
  
  return { timestamp: new Date().toISOString() };
}

export function teardown(data) {
  console.log('🏁 Performance Tests Completed');
  console.log(`Started at: ${data.timestamp}`);
  console.log(`Ended at: ${new Date().toISOString()}`);
  
  // Generate summary report
  console.log('\n📊 Test Summary:');
  console.log(`- Error Rate: ${errorRate.rate * 100}%`);
  console.log(`- Average Response Time: ${responseTime.avg}ms`);
  console.log(`- WebSocket Connections: ${wsConnections.count}`);
  console.log(`- WebSocket Messages: ${wsMessages.count}`);
}

// =====================================================
// SLO VALIDATION SCRIPT
// =====================================================

// Ubicación: scripts/validate-slos.js
export const sloValidation = `
const fs = require('fs');
const path = require('path');

function validateSLOs(resultsFile) {
  console.log('📊 Validating SLOs against performance results...');
  
  const results = JSON.parse(fs.readFileSync(resultsFile, 'utf8'));
  const metrics = results.metrics;
  
  const slos = {
    'Response Time P95': { 
      actual: metrics.http_req_duration?.values?.['p(95)'], 
      target: 500, 
      unit: 'ms' 
    },
    'Error Rate': { 
      actual: metrics.http_req_failed?.values?.rate * 100, 
      target: 1, 
      unit: '%' 
    },
    'Availability': { 
      actual: (1 - metrics.http_req_failed?.values?.rate) * 100, 
      target: 99.9, 
      unit: '%' 
    },
    'WebSocket Connection Success': {
      actual: metrics.websocket_connections?.values?.count || 0,
      target: 1,
      unit: 'connections'
    }
  };
  
  let allPassed = true;
  
  console.log('\\n🎯 SLO Validation Results:');
  console.log('================================');
  
  Object.entries(slos).forEach(([name, slo]) => {
    const passed = slo.actual <= slo.target || (name === 'Availability' && slo.actual >= slo.target);
    const status = passed ? '✅ PASS' : '❌ FAIL';
    
    console.log(\`\${status} \${name}: \${slo.actual}\${slo.unit} (target: \${slo.target}\${slo.unit})\`);
    
    if (!passed) {
      allPassed = false;
    }
  });
  
  console.log('================================');
  
  if (allPassed) {
    console.log('🎉 All SLOs passed!');
    process.exit(0);
  } else {
    console.log('💥 Some SLOs failed!');
    process.exit(1);
  }
}

// Run validation if called directly
if (require.main === module) {
  const resultsFile = process.argv[2];
  if (!resultsFile) {
    console.error('Usage: node validate-slos.js <results-file.json>');
    process.exit(1);
  }
  
  validateSLOs(resultsFile);
}

module.exports = { validateSLOs };
`;

// =====================================================
// PERFORMANCE TEST COMMANDS
// =====================================================

/*
# Backend Load Test
k6 run --env BASE_URL=https://api.arbitragex.dev --env TEST_TYPE=load performance/load-test.js

# Edge Load Test  
k6 run --env EDGE_URL=https://arbitragex.workers.dev --env TEST_TYPE=load performance/edge-load-test.js

# WebSocket Test
k6 run --env WS_URL=wss://arbitragex.workers.dev/ws --env TEST_TYPE=websocket performance/load-test.js

# Stress Test
k6 run --env BASE_URL=https://api.arbitragex.dev --env TEST_TYPE=stress performance/load-test.js

# Spike Test
k6 run --env BASE_URL=https://api.arbitragex.dev --env TEST_TYPE=spike performance/load-test.js

# Full Test Suite
k6 run --out json=performance-results.json performance/load-test.js
node scripts/validate-slos.js performance-results.json
*/
