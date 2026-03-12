# API Reference

## Endpoints

### POST /api/auth/login
Authenticates a user and returns a session token.

Request body:
```json
{
  "username": "string",
  "password": "string"
}
```

Response (200):
```json
{
  "token": "jwt-token-here",
  "user": {
    "id": 123,
    "username": "john",
    "role": "admin"
  }
}
```

### GET /api/users
Returns a list of all users. Requires admin role.

### POST /api/tickets
Creates a new support ticket.

Request body:
```json
{
  "title": "string",
  "description": "string",
  "priority": "low|medium|high|critical"
}
```

### GET /api/tickets/:id
Returns a specific ticket with its history.

### PUT /api/tickets/:id/resolve
Marks a ticket as resolved.
