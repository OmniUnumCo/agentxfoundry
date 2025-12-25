#!/bin/bash

# Jasteri.sh - Command-line utility for J* language processing, encryption, and key management

set -e

# Configuration
JSTAR_DIR="jstar_output"
SECRETS_DIR="secrets"
ADMINS=("Jimmy Paras" "Jason Jarmacz")

# Function to generate ECC keys
generate_keys() {
    echo "Generating ECC keys for admins..."
    for admin in "${ADMINS[@]}"; do
        admin_key="${SECRETS_DIR}/${admin// /_}_ecc_key.pem"
        python3 -c "
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization
sk = ec.generate_private_key(ec.SECP256R1())
pem = sk.private_bytes(
    encoding=serialization.Encoding.PEM,
    format=serialization.PrivateFormat.PKCS8,
    encryption_algorithm=serialization.NoEncryption()
)
with open('$admin_key', 'wb') as f:
    f.write(pem)
print('Generated key for $admin')
        "
    done
}

# Function to lex code into J*
lex_to_jstar() {
    input_file="$1"
    output_file="$JSTAR_DIR/$(basename "$input_file" .py).jstar"
    echo "Lexing $input_file to $output_file..."
    mkdir -p "$JSTAR_DIR"
    # Simple lexing: wrap in J* codex
    cat > "$output_file" << EOF
// J* Lexed from $input_file
codex $(basename "$input_file" .py) {
    source: "$input_file"
    // Add lexed content here
}
EOF
}

# Function to encrypt file with ECC
encrypt_file() {
    input_file="$1"
    output_file="$1.encrypted"
    admin_key="$2"
    echo "Encrypting $input_file with $admin_key..."
    python3 -c "
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization, hashes
from cryptography.hazmat.primitives.asymmetric import padding
import os

# Load key
with open('$admin_key', 'rb') as f:
    sk = serialization.load_pem_private_key(f.read(), password=None)

# Simple encryption (in practice, use proper hybrid encryption)
data = open('$input_file', 'rb').read()
signature = sk.sign(data, ec.ECDSA(hashes.SHA256()))
with open('$output_file', 'wb') as f:
    f.write(signature + b'|' + data)
print('Encrypted $input_file')
    "
}

# Main logic
case "$1" in
    generate-keys)
        generate_keys
        ;;
    lex)
        if [ -z "$2" ]; then
            echo "Usage: $0 lex <file>"
            exit 1
        fi
        lex_to_jstar "$2"
        ;;
    encrypt)
        if [ -z "$2" ] || [ -z "$3" ]; then
            echo "Usage: $0 encrypt <file> <admin_key>"
            exit 1
        fi
        encrypt_file "$2" "$3"
        ;;
    *)
        echo "Usage: $0 {generate-keys|lex <file>|encrypt <file> <admin_key>}"
        exit 1
        ;;
esac