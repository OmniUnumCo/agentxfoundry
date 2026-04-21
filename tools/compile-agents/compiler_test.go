package main

import (
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"text/template"
)

// buildDoctrineTree creates a minimal doctrine directory structure for tests.
// Returns the doctrine root path.
func buildDoctrineTree(t *testing.T, mosFile, mosContent, rankFile, rankContent string) string {
	t.Helper()
	root := t.TempDir()

	for _, sub := range []string{"mos", "ranks", "protocols"} {
		if err := os.MkdirAll(filepath.Join(root, sub), 0755); err != nil {
			t.Fatalf("mkdir %s: %v", sub, err)
		}
	}

	write := func(path, content string) {
		if err := os.WriteFile(path, []byte(content), 0644); err != nil {
			t.Fatalf("write %s: %v", path, err)
		}
	}

	write(filepath.Join(root, "mos", mosFile), mosContent)
	write(filepath.Join(root, "ranks", rankFile), rankContent)
	write(filepath.Join(root, "protocols", "sitrep.md"), "# SITREP\nReport status upward.")
	write(filepath.Join(root, "protocols", "casrep.md"), "# CASREP\nReport casualties immediately.")
	write(filepath.Join(root, "protocols", "frago.md"), "# FRAGO\nFragmentary order format.")
	write(filepath.Join(root, "protocols", "aar.md"), "# AAR\nAfter action review.")

	return root
}

// buildTemplate creates a minimal agent template, parses it, and returns the
// *template.Template along with the file path (for compileAll which reads by path).
func buildTemplate(t *testing.T) (string, *template.Template) {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "agent-template.md")
	content := `---
name: RR {{.MOSCode}} {{.MOSName}}
description: {{.Description}}
model: {{.Model}}
---

# Identity

You are a {{.RankName}} ({{.RankCode}}) in the Rational Reserve with MOS {{.MOSCode}} ({{.MOSName}}).
Category: {{.Category}}
Authority level: {{.AuthorityLevel}}/12

## Decision Posture

{{.DecisionPosture}}

# Specialty Doctrine

{{.MOSDoctrine}}

# Communication Protocols

## SITREP

{{.SitrepProtocol}}

## CASREP

{{.CasrepProtocol}}

## FRAGO

{{.FragoProtocol}}

## AAR

{{.AARProtocol}}
`
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write template: %v", err)
	}
	tmpl, err := template.New("agent").Parse(content)
	if err != nil {
		t.Fatalf("parse template: %v", err)
	}
	return path, tmpl
}

// TestExtractDecisionPosture verifies that posture lines are extracted and
// the rank heading line itself is not included.
func TestExtractDecisionPosture(t *testing.T) {
	rankContent := `# Specialist (SPC) -- Authority 3

## Role
Technical expert on the execution line.

## Decision posture
- Do the work the MOS doctrine says to do.
- Be honest about progress.

## Authority
- No command authority.
`
	result := extractDecisionPosture(rankContent)

	if !strings.Contains(result, "Do the work the MOS doctrine says to do.") {
		t.Errorf("expected posture line in result; got: %q", result)
	}
	if !strings.Contains(result, "Be honest about progress.") {
		t.Errorf("expected second posture line in result; got: %q", result)
	}
	if strings.Contains(result, "Decision posture") {
		t.Errorf("heading should be excluded from result; got: %q", result)
	}
	if strings.Contains(result, "## Role") {
		t.Errorf("preceding section should not appear in result; got: %q", result)
	}
	if strings.Contains(result, "## Authority") {
		t.Errorf("following section should not appear in result; got: %q", result)
	}
}

// TestCompileAgent verifies that a compiled agent file has correct frontmatter,
// identity block, decision posture, MOS doctrine, and protocol content.
func TestCompileAgent(t *testing.T) {
	rankContent := `# Specialist (SPC) -- Authority 3

## Decision posture
- Do the work the MOS doctrine says to do.
- Be honest about progress.
`
	mosContent := `# 11B -- Infantry

## Core competencies
- General-purpose execution.
`
	doctrineDir := buildDoctrineTree(t, "11b-infantry.md", mosContent, "specialist.md", rankContent)
	_, tmpl := buildTemplate(t)
	outDir := t.TempDir()

	entry := MOSEntry{
		Code:         "11B",
		Name:         "Infantry",
		Category:     "OPS",
		DefaultRank:  "SPC",
		RankName:     "Specialist",
		AuthLevel:    3,
		Model:        "sonnet",
		DoctrineFile: "11b-infantry.md",
		RankFile:     "specialist.md",
		AgentFile:    "rr-11b-infantry.md",
		Description:  "General-purpose task execution",
	}

	protocols, err := loadProtocols(doctrineDir)
	if err != nil {
		t.Fatalf("loadProtocols: %v", err)
	}

	rendered, err := compileAgentToBytes(entry, doctrineDir, tmpl, protocols)
	if err != nil {
		t.Fatalf("compileAgentToBytes returned error: %v", err)
	}

	outPath := filepath.Join(outDir, entry.AgentFile)
	if err := os.WriteFile(outPath, rendered, 0644); err != nil {
		t.Fatalf("write output file: %v", err)
	}

	data, err := os.ReadFile(outPath)
	if err != nil {
		t.Fatalf("output file not found: %v", err)
	}
	content := string(data)

	// Frontmatter checks
	if !strings.Contains(content, "name: RR 11B Infantry") {
		t.Errorf("missing frontmatter name; content:\n%s", content)
	}
	if !strings.Contains(content, "model: sonnet") {
		t.Errorf("missing frontmatter model; content:\n%s", content)
	}

	// Identity block checks
	if !strings.Contains(content, "Authority level: 3/12") {
		t.Errorf("missing authority level; content:\n%s", content)
	}

	// Decision posture check
	if !strings.Contains(content, "Do the work the MOS doctrine says to do.") {
		t.Errorf("missing decision posture content; content:\n%s", content)
	}

	// MOS doctrine check
	if !strings.Contains(content, "General-purpose execution.") {
		t.Errorf("missing MOS doctrine content; content:\n%s", content)
	}

	// Protocol check
	if !strings.Contains(content, "SITREP") {
		t.Errorf("missing SITREP section; content:\n%s", content)
	}
}

// TestCompileAgentMissingDoctrine verifies that a missing MOS doctrine file
// causes compileAgentToBytes to return a non-nil error.
func TestCompileAgentMissingDoctrine(t *testing.T) {
	rankContent := `# Specialist (SPC) -- Authority 3

## Decision posture
- Do the work.
`
	// Build tree but with a different MOS file than the entry expects.
	doctrineDir := buildDoctrineTree(t, "other-mos.md", "content", "specialist.md", rankContent)
	_, tmpl := buildTemplate(t)

	entry := MOSEntry{
		Code:         "11B",
		Name:         "Infantry",
		Category:     "OPS",
		DefaultRank:  "SPC",
		RankName:     "Specialist",
		AuthLevel:    3,
		Model:        "sonnet",
		DoctrineFile: "11b-infantry.md", // this file does NOT exist in the tree
		RankFile:     "specialist.md",
		AgentFile:    "rr-11b-infantry.md",
		Description:  "General-purpose task execution",
	}

	protocols, err := loadProtocols(doctrineDir)
	if err != nil {
		t.Fatalf("loadProtocols: %v", err)
	}

	_, err = compileAgentToBytes(entry, doctrineDir, tmpl, protocols)
	if err == nil {
		t.Fatal("expected error for missing doctrine file, got nil")
	}
}

// TestCompileAllIdempotent verifies that the first run writes 1 file and
// a second run (with identical content) writes 0 files.
func TestCompileAllIdempotent(t *testing.T) {
	rankContent := `# Specialist (SPC) -- Authority 3

## Decision posture
- Do the work the MOS doctrine says to do.
`
	mosContent := `# 11B -- Infantry

## Core competencies
- General-purpose execution.
`
	doctrineDir := buildDoctrineTree(t, "11b-infantry.md", mosContent, "specialist.md", rankContent)
	templatePath, _ := buildTemplate(t)
	outDir := t.TempDir()

	table := []MOSEntry{
		{
			Code:         "11B",
			Name:         "Infantry",
			Category:     "OPS",
			DefaultRank:  "SPC",
			RankName:     "Specialist",
			AuthLevel:    3,
			Model:        "sonnet",
			DoctrineFile: "11b-infantry.md",
			RankFile:     "specialist.md",
			AgentFile:    "rr-11b-infantry.md",
			Description:  "General-purpose task execution",
		},
	}

	// First run -- should write 1 file.
	n, err := compileAll(table, doctrineDir, templatePath, outDir, io.Discard)
	if err != nil {
		t.Fatalf("first compileAll error: %v", err)
	}
	if n != 1 {
		t.Errorf("first run: expected 1 file written, got %d", n)
	}

	// Second run -- content unchanged, should write 0 files.
	n, err = compileAll(table, doctrineDir, templatePath, outDir, io.Discard)
	if err != nil {
		t.Fatalf("second compileAll error: %v", err)
	}
	if n != 0 {
		t.Errorf("second run: expected 0 files written, got %d", n)
	}
}
