# carve-api: implements the backend api for carve-web.

It implements the following api:
---
openapi: 3.0.4
info:
  title: Carve API
  description: implements the backend api for carve-web.
  version: 0.0.1

servers:
  - url: http://api.example.carve.com/api/v1
paths:
  /user:
    get:
      summary: Returns information about a user
        parameters:
        - name: id
        in: query
        description: The ID of the user to retrieve
        required: true
        schema:
            type: number
            example: 12345
        responses:
            '200':
                description: A user object
                content:
                application/json:
                    schema:
                    type: object
                    properties:
                        id:
                            type: number
                            example: 12345
                        name:
                            type: string
                            example: "myname"
                        email:
                            type: string
                            example: "myname@example.com"
                        teamId:
                            type: number
                            example: 12345
            '404':
                description: User not found
    /team:
      get:
        summary: Returns information about a team
        parameters:
        - name: id
            in: query
            description: The ID of the team to retrieve
            required: true
            schema:
            type: number
            example: 12345
        responses:
            '200':
                description: A team object
                content:
                application/json:
                    schema:
                    type: object
                    properties:
                        id:
                            type: number
                            example: 12345
                        name:
                            type: string
                            example: "myteam"
                        members:
                            type: array
                            items:
                                type: object
                                properties:
                                id:
                                    type: number
                                    example: 12345
                                name:
                                    type: string
                                    example: "membername"
            '404':
                description: Team not found
    /competition:
        get:
            summary: Returns information about the current competition
            responses:
                '200':
                    description: A competition object
                    content:
                    application/json:
                        schema:
                        type: object
                        properties:
                            status:
                                type: string
                                example: "active"
                            name:
                                type: string
                                example: "My Competition"
                            startDate:
                                type: string
                                format: date-time
                                example: "2023-10-01T00:00:00Z"
                            endDate:
                                type: string
                                format: date-time
                                example: "2023-10-31T23:59:59Z"
                '404':
                    description: Competition not found
    /score:
        get:
            summary: Returns an array of successful/unsuccessful check events for a team over time, with optional filters, by team ID, scoring check, and date/time range.
            parameters:
            - name: teamId
              in: query
              description: The ID of the team to retrieve scores for
              required: false
              schema:
                type: number
                example: 12345
            - name: scoringCheck
                in: query
                description: The scoring check to filter by
                required: false
                schema:
                    type: string
                    example: "check1"
            - name: startDate
                in: query
                description: The start date/time to filter by (ISO 8601 format)
                required: false
                schema:
                    type: string
                    format: date-time
                    example: "2023-10-01T00:00:00Z"
            - name: endDate
                in: query
                description: The end date/time to filter by (ISO 8601 format)
                required: false
                schema:
                    type: string
                    format: date-time
                    example: "2023-10-31T23:59:59Z"
            responses:
                '200':
                    description: An array of score objects
                    content:
                    application/json:
                        schema:
                        type: array
                        items:
                            type: object
                            properties:
                                id:
                                    type: number
                                    example: 12345
                                teamId:
                                    type: number
                                    example: 12345
                                scoringCheck:
                                    type: string
                                    example: "check1"
                                timestamp:
                                    type: string
                                    format: date-time
                                    example: "2023-10-01T00:00:00Z"
                '404':
                    description: Scores not found
    /score/leaderboard:
        get:
            summary: Returns the leaderboard for a competition
            responses:
                '200':
                    description: A leaderboard object
                    content:
                    application/json:
                        schema:
                        type: object
                        properties:
                            teams:
                                type: array
                                items:
                                    type: object
                                    properties:
                                        teamId:
                                            type: number
                                            example: 12345
                                        score:
                                            type: number
                                            example: 1000
                                        rank:
                                            type: number
                                            example: 1
    /boxes:
        get:
            summary: Returns a list of boxes for a team, and their boxIds and names.
            parameters:
            - name: teamId
                in: query
                description: The ID of the team to retrieve boxes for
                required: true
                schema:
                    type: number
                    example: 12345
            responses:
                '200':
                    description: An array of box objects
                    content:
                    application/json:
                        schema:
                        type: array
                        items:
                            type: object
                            properties:
                                name:
                                    type: string
                                    example: "web.team1.competition.hack"
                '404':
                    description: Boxes/team not found
    /box:
        get:
            summary: Returns information about a specific box by its name.
            parameters:
            - name: name
                in: query
                description: The name of the box to retrieve
                required: true
                schema:
                    type: string
                    example: "web.team1.competition.hack"
            responses:
                '200':
                    description: A box object
                    content:
                    application/json:
                        schema:
                        type: object
                        properties:
                            name:
                                type: string
                                example: "web.team1.competition.hack"
                            ipAddress:
                                type: string
                                example: "192.168.1.1"
                            status:
                                type: string
                                example: "active"
                '404':
                    description: Box not found
    /box/defaultCreds:
        get:
            summary: Returns the default credentials for a specific box by its name.
            parameters:
            - name: name
                in: query
                description: The name of the box to retrieve the default credentials for
                required: true
                schema:
                    type: string
                    example: "web.team1.competition.hack"
            responses:
                '200':
                    description: A box default credentials object
                    content:
                    application/json:
                        schema:
                        type: object
                        properties:
                            name:
                                type: string
                                example: "web.team1.competition.hack"
                            username:
                                type: string
                                example: "admin"
                            password:
                                type: string
                                example: "password123"
                '404':
                    description: Creds not set.
    /checks:
        get:
            summary: Returns a list of the checks in the competition.
            responses:
                '200':
                    description: An array of check objects
                    content:
                    application/json:
                        schema:
                        type: array
                        items:
                            type: object
                            properties:
                                name:
                                    type: string
                                    example: "Check 1"
                                points:
                                    type: number
                                    example: 100
                '404':
                    description: Checks not found

There is also a oauth2 callback url at api/v1/oauth2/callback