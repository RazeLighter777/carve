# Canary: Competition Scoring Service

Canary is a service that conducts automated scoring checks for competitions. It monitors the health and functionality of competition targets by performing HTTP, ICMP, and SSH connection checks and recording the results in a Redis database.

## Key Features

* **Persistence**: Uses Redis as the database backend
* **Multiple Check Types**: Supports HTTP, ICMP, and SSH connection checks
* **Configuration-driven**: All settings defined in a YAML configuration file
* **Synchronized Checks**: Checks run at synchronized times when `(unix timestamp) mod interval == 0`
* **Health Monitoring**: HTTP health check endpoint at `/api/health`

## Check Types

1. **HTTP Checks**:
   * Verify that HTTP endpoints return specific status codes (e.g., 200)
   * Verify response contents with regex patterns

2. **ICMP Checks**:
   * Basic ping connectivity checks
   * Verify expected ICMP response codes

3. **SSH Checks**:
   * Verify SSH login functionality
   * Support for password or key-based authentication

## How It Works

* Checks run at synchronized intervals across replicas
* Results are stored in Redis streams using the format: `<competition name>:<check name>:<team name>`
* If ANY replica succeeds, the check succeeds
* All check results are written to Redis with timestamps, box info, and status

## Usage

1. Create a `competition.yaml` file in the working directory:

```yaml
competitions:
  - name: mycompetition
    redis:
      host: redis.server.local
      port: 6379
      db: 0
    boxes:
      - name: web-server
        labels: http
        hostname: "{{ .TEAM }}-web.local"
    teams:
      - name: team1
      - name: team2
    checks:
      - name: http-check
        interval: 30
        points: 5
        labelSelector:
          "": http
        spec:
          type: http
          url: /status
          code: 200
          regex: "online"
```

2. Run the application:

```bash
cargo run
```

The application will:
- Read the configuration from `competition.yaml`
- Connect to the specified Redis instance
- Start a scheduler for each competition defined in the configuration
- Run checks at the specified intervals
- Record check results in Redis streams
- Provide a health check endpoint at `/api/health`

## Redis Data Format

Results are stored in Redis streams with the following format:

```
XADD <competition name>:<check> <miliseconds of check>-* result 1|0 team "team-name" box "box-name" message "message"
```

Where:
- `miliseconds of check` is aligned to the check interval
- `result` is 1 for success, 0 for failure
- Each entry includes the team name, box name, and a message

## Health Check

The service provides a health check endpoint at `/api/health` that returns:
- HTTP 200 if the Redis connection is healthy
- HTTP 500 if there are any Redis connection issues
## Paramaters
Configurable through competition.yaml in working directory. (uses config-rs with yaml feature flag)

Example:

competitions:
- name: defcon
  redis:
    host: redis.carve.svc.cluster.local
    port: 6379
    db:0
  boxes:
  - name: web-server
    labels: http
    hostname: {{ .TEAM }}-web-server.carve.svc.cluster.local
  - name: db-server
    labels: db
    hostname: {{ .TEAM }}-db.carve.svc.cluster.local
  teams:
  - name: team1
  - name: team2
  checks:
  - name: http-example 
    interval: 15
    points: 5
    labelSelector: http
    spec:
      type: http
      url: /api/ping
      code: 200
      regex: pong
  - name: icmp-example
    labelSelector: {} # all competition pods
    interval: 30
    points: 1
    spec:
      type: icmp
      code: 0 # echo reply


### Example insert

XADD <competition name>:<check> <miliseconds of check>-* result 1 team "team-1" box "web-server"  message "HTTP server success"

Note that the miliseconds should be when (unix timestamp) mod interval == 0 so all instances have the same timestamp, but different entry IDs.

