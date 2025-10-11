# API Proxy

A high-performance, multi-region HTTP and SOAP proxy built with Rust for Cloudflare Workers using Durable Objects. Designed for reliable API forwarding with built-in authorization, intelligent logging, and global distribution.

[![Deployed on Cloudflare Workers](https://img.shields.io/badge/Deployed%20on-Cloudflare%20Workers-orange)](https://workers.cloudflare.com/)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-red)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## ✨ Features

- **🔒 Secure Authorization**: Bearer token authentication via Cloudflare Secrets
- **🌍 Multi-Region Support**: 8 Cloudflare regions with WNAM default
- **📊 Two-Level Logging**: Info (production) and debug logging via `X-Log-Level` header
- **🚀 Pure Rust**: WASM-compiled for maximum performance (~325KB gzip)
- **🔄 HTTP Proxy**: Support for GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
- **🧼 SOAP Support**: NuSOAP-compatible SOAP 1.1 via `X-Request-Type: soap` header
- **⚡ Edge Performance**: Sub-millisecond startup, global deployment with Durable Objects
- **🛡️ EU Jurisdiction**: Location hints ensure GDPR-compliant data residency
- **📝 Smart Logging**: Minimal logging (info) by default, detailed logging on demand (debug)

## 💡 Why I Built This

I ran into a real-world problem: my server's region was blocked by an API provider I needed to integrate with. Traditional solutions didn't work:

- **VPN wasn't practical** - I only needed it for specific API calls, not all traffic
- **IP whitelisting conflicts** - Some of my integrations had strict IP whitelists that wouldn't work with a VPN
- **Multiple regional requirements** - Different providers needed different regions (some required EU for GDPR, others needed US endpoints)

This proxy solves these problems by:
- **Selective routing** - Route only specific requests through the proxy, not all traffic
- **Multi-region flexibility** - Choose the appropriate region per request via headers
- **Stable IP addresses** - Cloudflare's Durable Objects provide consistent IPs per region
- **Zero server maintenance** - Runs on Cloudflare's edge network with automatic scaling

## 🚀 Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Node.js](https://nodejs.org/) (for Wrangler)
- [Cloudflare account](https://dash.cloudflare.com/sign-up)
- [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/install-and-update/)

### Installation

1. **Clone the repository**
   ```bash
   git clone <your-repo-url>
   cd api-proxy
   ```

2. **Install dependencies**
   ```bash
   npm install -g wrangler
   cargo install worker-build
   ```

3. **Login to Cloudflare**
   ```bash
   wrangler login
   ```

4. **Configure AUTH_TOKEN**
   ```bash
   # Generate secure token
   openssl rand -base64 32

   # Set as Cloudflare secret
   wrangler secret put AUTH_TOKEN
   # Paste the generated token when prompted
   ```

5. **Deploy**
   ```bash
   wrangler deploy
   ```

## 🔧 Usage

### HTTP Proxy Request

**Basic POST Request**
```bash
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://api.example.com/data",
    "method": "post",
    "params": {
      "key": "value"
    }
  }'
```

**GET Request with Query Parameters**
```bash
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://api.example.com/users",
    "method": "get",
    "params": {
      "page": "1",
      "limit": "10"
    }
  }'
```

**Response Format**
```json
{
  "status": 200,
  "headers": {
    "content-type": "application/json"
  },
  "body": {
    "result": "success"
  }
}
```

### SOAP Request

**SOAP Request with X-Request-Type Header**
```bash
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -H "X-Request-Type: soap" \
  -d '{
    "url": "https://soap.example.com/service",
    "action": "getDIDCountry",
    "namespace": "urn:getDIDCountry",
    "params": [
      ["did", "1234567890"],
      ["country", "US"]
    ]
  }'
```

## 🌍 Multi-Region Support

Control request processing location with the `X-CF-Region` header.

### Available Regions

| Region Code | Description | Location Hint |
|-------------|-------------|---------------|
| `wnam` | Western North America | US West |
| `enam` | Eastern North America | US East |
| `weur` | Western Europe | EU West (GDPR) |
| `eeur` | Eastern Europe | EU East (GDPR) |
| `apac` | Asia-Pacific | Asia |
| `oc` | Oceania | Australia |
| `af` | Africa | Africa |
| `me` | Middle East | Middle East |
| **Default** | *(no header)* | **WNAM** |

### Region Example

```bash
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_AUTH_TOKEN" \
  -H "X-CF-Region: weur" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://api.example.com/data", "method": "get"}'
```

**Log Output (with debug logging):**
```
[INFO] Request received at datacenter: CDG
[INFO] Selected region: weur
[INFO] WEURProcessor processing in datacenter: CDG (Region: Western Europe)
[INFO] Processing HTTP request
[DEBUG] HTTP method: Get, url: https://api.example.com/data
[INFO] Response status: 200
[INFO] HTTP request completed successfully
```

## 📊 Logging

Two logging levels controlled via `X-Log-Level` header.

### Log Levels

| Level | Default | What's Logged |
|-------|---------|---------------|
| `info` | ✅ Yes | Request received, region selected, DO processing location, response status, completion |
| `debug` | ⬜ No | All of the above PLUS: request path, HTTP method, target URL, parameters count, headers count, response body size |

### Info Logging (Production - Default)

```bash
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://httpbin.org/post", "method": "post", "params": {"test": "value"}}'
```

**Log Output:**
```
[INFO] Request received at datacenter: LAX
[INFO] Selected region: wnam
[INFO] WNAMProcessor processing in datacenter: LAX (Region: Western North America)
[INFO] Processing HTTP request
[INFO] Response status: 200
[INFO] HTTP request completed successfully
```

### Debug Logging (Development/Testing)

```bash
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_AUTH_TOKEN" \
  -H "X-Log-Level: debug" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://httpbin.org/post", "method": "post", "params": {"test": "value"}}'
```

**Log Output:**
```
[INFO] Request received at datacenter: LAX
[DEBUG] Request path: /
[INFO] Selected region: wnam
[INFO] WNAMProcessor processing in datacenter: LAX (Region: Western North America)
[INFO] Processing HTTP request
[DEBUG] HTTP method: Post, url: https://httpbin.org/post
[DEBUG] Sending Post request to https://httpbin.org/post with JSON body
[DEBUG] Request headers: 0 custom headers
[INFO] Response status: 200
[DEBUG] Response headers: 8 headers
[DEBUG] Response body size: 730 bytes
[INFO] HTTP request completed successfully
```

## 🔐 Authorization

All requests require a valid `AUTH_TOKEN` in the `Authorization` header.

### Setting the AUTH_TOKEN

**Generate a secure token:**
```bash
openssl rand -base64 32
```

**Set as Cloudflare secret:**
```bash
wrangler secret put AUTH_TOKEN
# Paste your generated token when prompted
```

**Use in requests:**
```bash
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_GENERATED_TOKEN" \
  ...
```

### Authorization Responses

| Status | Condition | Response |
|--------|-----------|----------|
| `200 OK` | Valid request | Proxy response |
| `403 Forbidden` | Missing or invalid token | `"Forbidden"` |

## 📡 API Reference

### Request Schema

#### HTTP Proxy Request

```typescript
{
  "url": string,              // Target URL (required)
  "method": string,           // HTTP method: get, post, put, delete, patch, head, options (default: "post")
  "params": object,           // Query params (GET/HEAD/DELETE) or body params (POST/PUT/PATCH)
  "headers": object,          // Additional headers to forward
  "timeout": number           // Timeout in seconds (default: 30, not enforced in WASM)
}
```

#### SOAP Request

```typescript
{
  "url": string,              // SOAP endpoint URL (required)
  "action": string,           // SOAP action/method name (required)
  "namespace": string,        // SOAP action namespace (required)
  "params": [string, any][],  // Array of [key, value] tuples (preserves order)
  "headers": object,          // Additional headers to forward
  "timeout": number           // Timeout in seconds (default: 30, not enforced in WASM)
}
```

**Note**: Use `X-Request-Type: soap` header to indicate SOAP request

### Response Schema

#### Success Response

```typescript
{
  "status": number,           // HTTP status code (200-299)
  "headers": object,          // Response headers as key-value pairs
  "body": any                 // Response body (JSON object or string)
}
```

#### Error Response

```typescript
{
  "status": number,           // HTTP status code (400-599)
  "message": string           // Error message
}
```

### Request Headers

| Header | Required | Default | Description |
|--------|----------|---------|-------------|
| `Authorization` | ✅ Yes | - | Bearer token authentication |
| `Content-Type` | ✅ Yes | - | Must be `application/json` |
| `X-CF-Region` | ⬜ No | `wnam` | Target region code |
| `X-Request-Type` | ⬜ No | `http` | Set to `soap` for SOAP requests |
| `X-Log-Level` | ⬜ No | `info` | Set to `debug` for detailed logging |

## 📮 Postman Collection

A comprehensive Postman collection is included for testing and API exploration.

### Import Collection

1. Import [`API_Proxy.postman_collection.json`](./API_Proxy.postman_collection.json) into Postman
2. Configure collection variables:
   - `{{BASE_URL}}`: `https://api-proxy.admice.com`
   - `{{AUTH_TOKEN}}`: Your token from Cloudflare secrets
   - `{{REGION}}`: Desired region (default: `wnam`)

### Included Requests

- **✅ Quick Tests** (4 requests)
  - HTTP GET, POST with parameters
  - SOAP request
  - Region-specific request

- **🌍 Multi-Region Tests** (3 requests)
  - WNAM, WEUR, APAC with datacenter verification

- **📊 Logging Tests** (2 requests)
  - Info level (default)
  - Debug level (detailed)

- **❌ Error Handling** (2 requests)
  - Invalid authentication
  - Target endpoint errors

## 🛠️ Development

### Local Development

```bash
# Install dependencies
cargo build

# Run local dev server
wrangler dev

# Test locally (use test token for local development)
curl -X POST http://localhost:8787/ \
  -H "Authorization: Bearer test-token" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://httpbin.org/get", "method": "get"}'
```

### Build

```bash
# Build for production
wrangler build

# Check build output
ls -lh build/
```

### Testing

```bash
# Watch logs in real-time
wrangler tail --format pretty

# Test with debug logging
curl -X POST https://api-proxy.admice.com/ \
  -H "Authorization: Bearer YOUR_AUTH_TOKEN" \
  -H "X-Log-Level: debug" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://httpbin.org/get", "method": "get"}'
```

## 📦 Deployment

### Deploy to Production

```bash
wrangler deploy
```

### Update Secret

```bash
wrangler secret put AUTH_TOKEN
```

### View Live Logs

```bash
wrangler tail --format pretty
```

## 🔄 CI/CD Setup

### Automatic Deployments via Cloudflare Dashboard

Cloudflare Workers provides built-in Git integration for automatic deployments. This is the easiest way to set up CI/CD for Rust WASM projects:

1. **Navigate to Cloudflare Dashboard**
   - Go to Workers & Pages
   - Select your `api-proxy` worker
   - Click Settings → Deployments

2. **Connect Git Repository**
   - Click "Connect to Git"
   - Authorize GitHub/GitLab
   - Select repository: `erimeilis/api-proxy`
   - Select branch: `main`

3. **Configure Build Settings**

   Since this is a Rust WASM project, the build configuration is not straightforward. Use these exact commands:

   **Build command:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   ```

   **Deploy command:**
   ```bash
   . "$HOME/.cargo/env" && npx wrangler deploy
   ```

   **Root directory:** `/` (default)

4. **Environment Variables**
   - Secrets (like `AUTH_TOKEN`) are managed separately via Wrangler CLI
   - They persist across deployments automatically
   - No need to configure in CI/CD settings

5. **Save and Deploy**
   - Click "Save and Deploy"
   - Every push to `main` will trigger automatic deployment
   - Build logs available in real-time during deployment

**Why these commands?**
- Rust isn't pre-installed in Cloudflare's build environment
- The build command installs Rust using the official rustup installer
- The deploy command sources the Rust environment and runs wrangler
- `worker-build` is automatically installed by the wrangler build process

## 🏗️ Architecture

```
    ┌─────────────────┐
    │  Client Request │
    │  (with token)   │
    └────────┬────────┘
             │
             ▼
┌─────────────────────────┐
│  Main Worker            │
│  ┌───────────────────┐  │
│  │ Authentication    │  │
│  └─────────┬─────────┘  │
│            │            │
│  ┌─────────▼─────────┐  │
│  │ Region Router     │  │ ← X-CF-Region header
│  │ (8 regions)       │  │
│  └─────────┬─────────┘  │
│            │            │
│  ┌─────────▼─────────┐  │
│  │ Log Level Setup   │  │ ← X-Log-Level header
│  └─────────┬─────────┘  │
└────────────┼────────────┘
             │
             ▼
   ┌────────────────────┐
   │ Durable Object     │
   │ (Regional DO)      │
   │ ┌────────────────┐ │
   │ │ Request Type   │ │ ← X-Request-Type header
   │ │ Routing        │ │
   │ └───────┬────────┘ │
   │         ├──────────┼─→ HTTP Handler
   │         └──────────┼─→ SOAP Handler
   └────────────────────┘
             │
             ▼
     ┌────────────────┐
     │  Target API    │
     │  (HTTP/SOAP)   │
     └────────────────┘
```

## 📊 Performance

- **Bundle Size**: ~914KB (325KB gzip)
- **Cold Start**: ~2ms
- **Execution**: Edge network via Durable Objects
- **Concurrent Requests**: Per-region isolation with Durable Objects
- **Logging Overhead**: Minimal (info level), ~5% additional for debug level

## 🔒 Security

- **AUTH_TOKEN**: Stored as Cloudflare secret (encrypted at rest)
- **HTTPS Only**: All requests over TLS 1.3
- **No Data Storage**: Stateless proxy, logs only to console
- **GDPR Compliant**: EU regions (weur/eeur) enforce EU datacenter execution
- **Location Hints**: Durable Objects placed in specified regions for data residency

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [workers-rs](https://github.com/cloudflare/workers-rs)
- Deployed on [Cloudflare Workers](https://workers.cloudflare.com/) with Durable Objects
- Designed for reliable, multi-region API proxying with intelligent logging

---

**Made with 💛💙 using Rust and Cloudflare Workers**
