import os
import secrets
from datetime import datetime, timedelta
from typing import Optional
from cryptography.fernet import Fernet
from passlib.context import CryptContext
from jose import JWTError, jwt
from fastapi import HTTPException, Depends, status
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from pydantic import BaseModel

# Load encryption key
def load_encryption_key() -> bytes:
    key_file = os.getenv("ENCRYPTION_KEY_FILE", "secrets/encryption_key")
    key_path = os.path.abspath(key_file)
    key_dir = os.path.dirname(key_path)

    # Ensure the target directory exists so key creation does not fail
    if key_dir and not os.path.exists(key_dir):
        os.makedirs(key_dir, exist_ok=True)

    if os.path.exists(key_path):
        with open(key_path, "rb") as f:
            return f.read()

    # Generate and persist a new key if it does not exist
    key = Fernet.generate_key()
    with open(key_path, "wb") as f:
        f.write(key)
    return key

fernet = Fernet(load_encryption_key())

# Password hashing
pwd_context = CryptContext(schemes=["bcrypt"], deprecated="auto")

# JWT settings
SECRET_KEY_FILE = os.getenv("JWT_SECRET_KEY_FILE", "secrets/jwt_secret_key")
jwt_env_secret = os.getenv("JWT_SECRET_KEY")
if os.path.exists(SECRET_KEY_FILE):
    with open(SECRET_KEY_FILE, 'r') as f:
        SECRET_KEY = f.read().strip()
elif jwt_env_secret:
    SECRET_KEY = jwt_env_secret.strip()
else:
    raise RuntimeError("JWT secret key is not configured. Set JWT_SECRET_KEY_FILE or JWT_SECRET_KEY.")
ALGORITHM = "HS256"
ACCESS_TOKEN_EXPIRE_MINUTES = 30

class TokenData(BaseModel):
    username: Optional[str] = None

security = HTTPBearer()

def verify_password(plain_password, hashed_password):
    return pwd_context.verify(plain_password, hashed_password)

def get_password_hash(password):
    return pwd_context.hash(password)

def create_access_token(data: dict, expires_delta: Optional[timedelta] = None):
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.utcnow() + expires_delta
    else:
        expire = datetime.utcnow() + timedelta(minutes=15)
    to_encode.update({"exp": expire})
    encoded_jwt = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return encoded_jwt

def verify_token(credentials: HTTPAuthorizationCredentials = Depends(security)):
    try:
        payload = jwt.decode(credentials.credentials, SECRET_KEY, algorithms=[ALGORITHM])
        username: str = payload.get("sub")
        if username is None:
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token")
        return TokenData(username=username)
    except JWTError:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token")

def encrypt_data(data: str) -> str:
    return fernet.encrypt(data.encode()).decode()

def decrypt_data(encrypted_data: str) -> str:
    return fernet.decrypt(encrypted_data.encode()).decode()

# Zero-trust: Assume breach, continuous verification
def continuous_verification(user_id: str, action: str) -> bool:
    # Placeholder for continuous verification logic
    # In practice, check user context, device, location, etc.
    return True  # Allow for now

# OSI Layer protections (high-level)
# Physical: Assume handled by infrastructure
# Data Link: MAC security (placeholder)
# Network: Firewall rules (placeholder)
# Transport: TLS encryption (use HTTPS)
# Session: Session management (JWT)
# Presentation: Data encryption (Fernet)
# Application: API authentication/authorization (JWT, RBAC)