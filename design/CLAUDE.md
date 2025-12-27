## design/CLAUDE.md
This file is used for guidance of AG-Hub Project in in domain of "design" (all contents under path: design/) and output standardized design documents for other domain to consume, for example, domain implementation.
If there is any trade-off, please record it in this file and raise the open discussion to the user.

# Sub-folder Path: design/decision/
Discussion on architecture design and decision. Contains:
- ADR as architectural decision records, file naming as ADR-{topic}.md
- FDR as functional decision records, file naming as FDR-{topic}.md
- PRD as formal product requirements documents, file naming as PRD-{topic}.md
- Each document should be linked to the corresponding ADR/FDR/PRD, and recorded in PRD.md under repo root path as index.

# Sub-folder Path: design/draft/
Initial Draft design documents. Contains:
- Semantic Code Engine Design, file naming as semantic-code-engine-design.md
- CodeMate Technical Specification, file naming as codemate-technical-specification.md

# Testing Principles: codemate/TESTING.md
Test Summary:
14 unit tests - Run after each task
5 E2E tests - Run before merge/commit
When new code module is added, check the coverage of unit tests and e2e tests. Add new tests if necessary and update the testing summary for guidelines.