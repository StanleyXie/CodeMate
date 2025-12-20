# CLAUDE.md

## Mandatory Principles

- Read and understand existing code before making modifications
- Never introduce security vulnerabilities (command injection, XSS, SQL injection, OWASP top 10)
- Keep solutions simple and focused - only make changes that are directly requested
- Do not add unnecessary features, refactoring, or "improvements" beyond what was asked

## Guardrails

- Never commit secrets, credentials, or sensitive data
- Never run destructive or irreversible commands without explicit user confirmation
- Never guess or fabricate information - investigate to find the truth
- Always validate assumptions before proceeding with significant changes

## Tooling for shell interactions 
Is it about finding FILES? use 'fd' 
Is it about finding TEXT/strings? use 'rg' 
Is it about finding CODE STRUCTURE? use 'ast-grep'
Is it about SELECTING from multiple results? pipe to 'fzf' 
Is it about interacting with JSON? use 'jq' 
Is it about interacting with YAML or XML? use 'yq'

## Key files
- AG-Hub/KRL.md # Key Requirements Log
- AG-Hub/PRD.md # Product Requirements Documents Index
- AG-Hub/TODO.md # TODO list for better follow up
