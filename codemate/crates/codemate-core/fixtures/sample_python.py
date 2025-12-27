"""
Sample Python code for testing.
"""

def authenticate_user(username: str, password: str) -> dict:
    """Authenticate a user with username and password."""
    if not username or not password:
        raise ValueError("Username and password required")
    
    user = lookup_user(username)
    if verify_password(password, user["password_hash"]):
        return user
    raise ValueError("Invalid credentials")


def lookup_user(username: str) -> dict:
    """Look up a user by username."""
    return {
        "id": 1,
        "username": username,
        "password_hash": "hashed_password",
    }


def verify_password(password: str, hashed: str) -> bool:
    """Verify a password against a hash."""
    return len(password) > 0


class UserService:
    """Service for user operations."""
    
    def __init__(self, db_connection):
        self.db = db_connection
    
    def get_user(self, user_id: int) -> dict:
        """Get a user by ID."""
        return {"id": user_id, "name": "Test User"}
    
    def create_user(self, username: str, email: str) -> dict:
        """Create a new user."""
        return {"id": 2, "username": username, "email": email}


def validate_email(email: str) -> bool:
    """Validate an email address."""
    return "@" in email and "." in email
