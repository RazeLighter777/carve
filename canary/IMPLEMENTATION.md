# Canary Implementation Summary

## Overview
Canary is a service that conducts automated scoring checks for competitions. It monitors the health and functionality of competition targets by performing HTTP, ICMP, and SSH connection checks and recording the results in a Redis database.

## Project Structure

1. **Configuration (`config.rs`)**: 
   - Defines structures for the application configuration
   - Loads and parses the YAML configuration file

2. **Check Implementation (`check.rs`)**:
   - Contains functions for different types of checks:
     - HTTP: Verifies HTTP endpoints and responses
     - ICMP: Performs ping checks for connectivity
     - SSH: Validates SSH login functionality

3. **Redis Manager (`redis_manager.rs`)**:
   - Handles connections to Redis
   - Provides functions to record check results
   - Implements health check functionality

4. **Scheduler (`scheduler.rs`)**:
   - Manages the timing of checks
   - Dispatches checks at synchronized intervals
   - Ensures checks run when `(unix timestamp) mod interval == 0`

5. **Main Application (`main.rs`)**:
   - Initializes the application
   - Sets up the Actix web server
   - Provides the health check endpoint

## Implementation Details

### Configuration
The application is configured using a YAML file (`competition.yaml`) that defines:
- Competitions
- Teams
- Target boxes
- Check specifications

### Check Types
1. **HTTP Checks**:
   - Verifies HTTP status codes
   - Validates response content using regex patterns

2. **ICMP Checks**:
   - Basic ping connectivity tests

3. **SSH Checks**:
   - Verifies SSH login functionality
   - Supports password and key-based authentication

### Redis Integration
- Results are stored in Redis streams
- Each check result includes:
  - Competition name
  - Check name
  - Timestamp (aligned to check interval)
  - Success/failure status
  - Team and box names
  - Message detailing the result

### Health Monitoring
- HTTP endpoint at `/api/health`
- Returns 200 if Redis connections are healthy
- Returns 500 if there are any connection issues

## Running the Application

1. Ensure Redis is available (or modify the connection details in `competition.yaml`)
2. Run the application:
   ```
   cargo run
   ```
3. The service will start performing checks according to the defined intervals

## Future Improvements

1. **Authentication**: Add authentication for the HTTP API
2. **Metrics**: Implement Prometheus metrics for monitoring
3. **Dashboard**: Create a web UI to view check results
4. **Notifications**: Add alert capabilities for check failures
