# Authentication Module

## Login Flow

The authentication system uses JWT tokens for session management.
When a user attempts to login, the system:

1. Validates the credentials against the database
2. Generates a JWT token with a 24-hour expiration
3. Sets the token in an HTTP-only cookie
4. Returns a success response with user profile data

## Error Handling

Common authentication errors include:
- `AUTH_INVALID_CREDENTIALS` - wrong username or password
- `AUTH_TOKEN_EXPIRED` - the JWT token has expired
- `AUTH_ACCOUNT_LOCKED` - too many failed login attempts

When an authentication error occurs, the system logs the event
and increments the failed attempt counter for the account.

## Password Reset

The password reset flow sends a one-time link via email.
The link expires after 15 minutes for security reasons.
