# Legacy / Archived Components

This directory contains files and systems that are not part of the core Rational Reserve system but are preserved for reference or potential future use.

## Contents

### Genkit Demo (TypeScript)
- **index.ts**: Character generation demo using Google Genkit
- **package.json**: Node.js dependencies for Genkit demo
- **package-lock.json**: Locked dependency versions

### Jasteri Language System
- **Jasteri.sh**: Custom language lexer and encryption utility
- **jstar_output/**: Output directory for lexed J* files

### Web Frontend (Separate Application)
- **public/**: Standalone web application
  - index.html: Web interface
  - script.js: Frontend logic
  - styles.css: Styling
  - README.md: Web app documentation

### Other
- **v2_nf_components.json**: Component configuration (legacy)

## Why These Are Archived

These components are separate from the Rational Reserve swarm system and are not required for its operation. They represent:

1. **Genkit Demo**: Alternative agent framework using TypeScript/Genkit
2. **Jasteri System**: Separate code lexing and encryption tool
3. **Web Frontend**: Standalone web application (unrelated to RR API)

## If You Need These

To use any legacy component:

1. **Genkit Demo**:
   ```bash
   npm install
   npm run genkit:dev
   ```

2. **Jasteri Utility**:
   ```bash
   ./Jasteri.sh generate-keys
   ./Jasteri.sh lex <file.py>
   ```

3. **Web Frontend**:
   - Serve public/ directory with any web server
   - See public/README.md for details

## Relation to Main Project

These systems were part of the original AgentX Foundry multi-system approach. The project has since focused on the Rational Reserve as the primary agentic swarm system.

The core RR system does NOT depend on any files in this legacy directory.
